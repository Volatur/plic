//! Хеш-таблица для виртуальной машины YG.
//!
//! Ключи – `ObjectRef`, хеш вычисляется через вызов `__hash__` на объекте,
//! сравнение – через `__eq__`. Все операции требуют явной передачи `Thread`.
//!
//! Для создания из итератора используйте `Map::from_iter(thread, iter)`.

use crate::napi::ptr::ObjectSmartRef;
use crate::std::core::{call_eq_or_eq, call_hash_or_nil};
use crate::vm::heap::ObjectRef;
use crate::vm::thread::VMThreadRef;
use crate::vm::VMError;

/// Собственная хеш-таблица с цепочками для разрешения коллизий.
pub struct Map {
    buckets: Vec<Vec<(ObjectRef, ObjectRef)>>,
    len: usize,
}

impl Map {
    /// Создаёт пустую таблицу с начальной ёмкостью 16 бакетов.
    pub fn new() -> Self {
        Self::with_capacity(16)
    }

    /// Создаёт таблицу с заданным количеством бакетов (не менее 1).
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(1);
        let buckets = (0..cap).map(|_| Vec::new()).collect();
        Map { buckets, len: 0 }
    }

    /// Создаёт таблицу из вектора пар (ключ, значение).
    pub fn from_vec(
        thread: VMThreadRef,
        vec: Vec<(ObjectRef, ObjectRef)>,
    ) -> Result<Self, VMError> {
        let mut map = Map::with_capacity(vec.len());
        for (k, v) in vec {
            map.insert(thread.clone(), k, v)?;
        }
        Ok(map)
    }

    /// Создаёт таблицу из произвольного итератора.
    pub fn from_iter<I: IntoIterator<Item = (ObjectRef, ObjectRef)>>(
        thread: VMThreadRef,
        iter: I,
    ) -> Result<Self, VMError> {
        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();
        let capacity = upper.unwrap_or(lower);
        let mut map = Map::with_capacity(capacity);
        map.extend_from_iter(thread, iter)?;
        Ok(map)
    }

    /// Вставляет пару (ключ, значение). Если ключ уже существовал, возвращает старое значение.
    pub fn insert(
        &mut self,
        thread: VMThreadRef,
        key: ObjectRef,
        value: ObjectRef,
    ) -> Result<Option<ObjectRef>, VMError> {
        if self.len as f64 / self.buckets.len() as f64 > 0.75 {
            self.resize(thread.clone())?;
        }

        let idx = self.bucket_index(thread.clone(), key)?;

        let bucket = &self.buckets[idx];
        let mut found_pos = None;
        for (pos, (k, _)) in bucket.iter().enumerate() {
            if self.keys_equal(thread.clone(), *k, key)? {
                found_pos = Some(pos);
                break;
            }
        }

        if let Some(pos) = found_pos {
            let bucket = &mut self.buckets[idx];
            let (_, old_value) = &mut bucket[pos];
            let old = std::mem::replace(old_value, value);
            Ok(Some(old))
        } else {
            let bucket = &mut self.buckets[idx];
            bucket.push((key, value));
            self.len += 1;
            Ok(None)
        }
    }

    /// Возвращает значение по ключу.
    pub fn get(
        &self,
        thread: VMThreadRef,
        key: ObjectRef,
    ) -> Result<Option<ObjectRef>, VMError> {
        let idx = self.bucket_index(thread.clone(), key)?;
        let bucket = &self.buckets[idx];
        for (k, v) in bucket {
            if self.keys_equal(thread.clone(), *k, key)? {
                return Ok(Some(*v));
            }
        }
        Ok(None)
    }

    /// Удаляет пару по ключу и возвращает значение.
    pub fn remove(
        &mut self,
        thread: VMThreadRef,
        key: ObjectRef,
    ) -> Result<Option<ObjectRef>, VMError> {
        let idx = self.bucket_index(thread.clone(), key)?;

        let bucket = &self.buckets[idx];
        let pos = bucket
            .iter()
            .position(|(k, _)| self.keys_equal(thread.clone(), *k, key).unwrap_or(false));

        if let Some(pos) = pos {
            let bucket = &mut self.buckets[idx];
            let (_, value) = bucket.swap_remove(pos);
            self.len -= 1;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Проверяет наличие ключа.
    pub fn contains_key(
        &self,
        thread: VMThreadRef,
        key: ObjectRef,
    ) -> Result<bool, VMError> {
        Ok(self.get(thread, key)?.is_some())
    }

    /// Расширяет таблицу элементами из итератора.
    pub fn extend_from_iter<I: IntoIterator<Item = (ObjectRef, ObjectRef)>>(
        &mut self,
        thread: VMThreadRef,
        iter: I,
    ) -> Result<(), VMError> {
        for (k, v) in iter {
            self.insert(thread.clone(), k, v)?;
        }
        Ok(())
    }

    /// Возвращает количество элементов.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Возвращает `true`, если таблица пуста.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Очищает таблицу.
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        self.len = 0;
    }

    // ---- Итераторы ----

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            buckets: &self.buckets,
            current_bucket: 0,
            current_index: 0,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            buckets: &mut self.buckets,
            current_bucket: 0,
            current_index: 0,
        }
    }

    pub fn into_iter(self) -> IntoIter {
        IntoIter {
            buckets: self.buckets,
            current_bucket: 0,
            current_index: 0,
        }
    }

    // ---- Вспомогательные методы ----

    fn bucket_index(&self, thread: VMThreadRef, key: ObjectRef) -> Result<usize, VMError> {
        let hash = self.hash_key(thread, key)?;
        Ok((hash as usize) % self.buckets.len())
    }

    fn hash_key(&self, thread: VMThreadRef, key: ObjectRef) -> Result<i64, VMError> {
        let key = ObjectSmartRef::new(key);
        call_hash_or_nil(thread, key)
    }

    fn keys_equal(&self, thread: VMThreadRef, key1: ObjectRef, key2: ObjectRef) -> Result<bool, VMError> {
        let key1 = ObjectSmartRef::new(key1);
        let key2 = ObjectSmartRef::new(key2);
        call_eq_or_eq(thread, key1, key2)
    }

    fn resize(&mut self, thread: VMThreadRef) -> Result<(), VMError> {
        let new_capacity = self.buckets.len() * 2;
        let mut entries = Vec::with_capacity(self.len);
        for bucket in &self.buckets {
            for (key, value) in bucket {
                let hash = self.hash_key(thread.clone(), *key)?;
                entries.push((*key, *value, hash));
            }
        }

        let mut new_buckets: Vec<Vec<(ObjectRef, ObjectRef)>> =
            (0..new_capacity).map(|_| Vec::new()).collect();

        for (key, value, hash) in entries {
            let idx = (hash as usize) % new_capacity;
            new_buckets[idx].push((key, value));
        }

        self.buckets = new_buckets;
        Ok(())
    }
}

// ---- Итераторы ----

pub struct Iter<'a> {
    buckets: &'a [Vec<(ObjectRef, ObjectRef)>],
    current_bucket: usize,
    current_index: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (ObjectRef, ObjectRef);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_bucket < self.buckets.len() {
            let bucket = &self.buckets[self.current_bucket];
            if self.current_index < bucket.len() {
                let (k, v) = bucket[self.current_index];
                self.current_index += 1;
                return Some((k, v));
            } else {
                self.current_bucket += 1;
                self.current_index = 0;
            }
        }
        None
    }
}

pub struct IterMut<'a> {
    buckets: &'a mut [Vec<(ObjectRef, ObjectRef)>],
    current_bucket: usize,
    current_index: usize,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = (ObjectRef, &'a mut ObjectRef);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_bucket < self.buckets.len() {
            let bucket = &mut self.buckets[self.current_bucket];
            if self.current_index < bucket.len() {
                let (k, v) = &mut bucket[self.current_index];
                let k_copy = *k;
                self.current_index += 1;
                let v_ptr = v as *mut ObjectRef;
                unsafe {
                    return Some((k_copy, &mut *v_ptr));
                }
            } else {
                self.current_bucket += 1;
                self.current_index = 0;
            }
        }
        None
    }
}

pub struct IntoIter {
    buckets: Vec<Vec<(ObjectRef, ObjectRef)>>,
    current_bucket: usize,
    current_index: usize,
}

impl Iterator for IntoIter {
    type Item = (ObjectRef, ObjectRef);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_bucket < self.buckets.len() {
            let bucket = &mut self.buckets[self.current_bucket];
            if self.current_index < bucket.len() {
                let (k, v) = bucket.swap_remove(self.current_index);
                return Some((k, v));
            } else {
                self.current_bucket += 1;
                self.current_index = 0;
            }
        }
        None
    }
}

// ---- Реализации IntoIterator ----

impl IntoIterator for Map {
    type Item = (ObjectRef, ObjectRef);
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a Map {
    type Item = (ObjectRef, ObjectRef);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Map {
    type Item = (ObjectRef, &'a mut ObjectRef);
    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}