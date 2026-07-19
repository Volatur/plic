//! Рекурсивный мьютекс с защитой данных.
//!
//! Предоставляет эксклюзивный доступ к значению с возможностью вложенных захватов.
//! Поддерживает автоматическое и ручное управление, а также неблокирующий захват.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering, fence};
use std::ops::{Deref, DerefMut};

/// Рекурсивный мьютекс, хранящий защищаемое значение `T`.
pub struct Mutex<T> {
    owner: AtomicUsize,
    count: AtomicUsize,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    /// Создаёт новый мьютекс с переданным значением.
    pub const fn new(value: T) -> Self {
        Self {
            owner: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
            data: UnsafeCell::new(value),
        }
    }

    /// Захватывает мьютекс и возвращает охранник для автоматического освобождения.
    /// Блокирует текущий поток, пока мьютекс занят другим потоком.
    ///
    /// # Panics
    /// Паникует, если `thread_id == 0`.
    pub fn lock(&self, thread_id: usize) -> MutexGuard<'_, T> {
        self.raw_lock(thread_id);
        MutexGuard { mutex: self, thread_id }
    }

    /// Пытается захватить мьютекс без блокировки.
    /// Возвращает `Some(MutexGuard)`, если удалось захватить, иначе `None`.
    ///
    /// # Panics
    /// Паникует, если `thread_id == 0`.
    pub fn try_lock(&self, thread_id: usize) -> Option<MutexGuard<'_, T>> {
        if self.try_raw_lock(thread_id) {
            Some(MutexGuard { mutex: self, thread_id })
        } else {
            None
        }
    }

    /// Захватывает мьютекс без создания охранника (блокирующий).
    pub fn raw_lock(&self, thread_id: usize) {
        assert_ne!(thread_id, 0, "thread_id cannot be 0");

        loop {
            let current = self.owner.load(Ordering::Acquire);

            if current == thread_id {
                self.count.fetch_add(1, Ordering::Relaxed);
                return;
            }

            if current == 0 {
                if self
                    .owner
                    .compare_exchange(0, thread_id, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    self.count.store(1, Ordering::Relaxed);
                    fence(Ordering::Acquire);
                    return;
                }
            } else {
                std::thread::yield_now();
            }
        }
    }

    /// Пытается захватить мьютекс без создания охранника (неблокирующий).
    pub fn try_raw_lock(&self, thread_id: usize) -> bool {
        assert_ne!(thread_id, 0, "thread_id cannot be 0");

        let current = self.owner.load(Ordering::Acquire);

        if current == thread_id {
            self.count.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        if current == 0 {
            if self
                .owner
                .compare_exchange(0, thread_id, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                self.count.store(1, Ordering::Relaxed);
                fence(Ordering::Acquire);
                return true;
            }
        }

        false
    }

    /// Освобождает мьютекс для указанного идентификатора потока.
    pub fn unlock(&self, thread_id: usize) {
        let current = self.owner.load(Ordering::Acquire);
        assert_eq!(current, thread_id, "unlock from non-owner thread");

        let prev = self.count.fetch_sub(1, Ordering::Relaxed);
        assert!(prev > 0, "count underflow");

        if prev == 1 {
            fence(Ordering::Release);
            self.owner.store(0, Ordering::Relaxed);
        }
    }

    /// Возвращает сырой указатель на внутренние данные.
    /// Может использоваться для unsafe операций без захвата мьютекса.
    ///
    /// # Safety
    /// Доступ через указатель должен быть синхронизирован с блокировками,
    /// иначе возможны гонки данных.
    pub fn data_ptr(&self) -> *mut T {
        self.data.get()
    }

    /// Возвращает изменяемую ссылку на данные, если мьютекс никем не захвачен.
    /// Для использования требуется `&mut self`, что гарантирует эксклюзивный доступ
    /// к самому мьютексу (и, следовательно, к данным) на время жизни ссылки.
    ///
    /// Это безопасно, так как наличие `&mut self` исключает наличие других ссылок
    /// на мьютекс, а значит, никто не может удерживать блокировку.
    pub fn get_mut(&mut self) -> &mut T {
        // Убеждаемся, что мьютекс свободен (owner == 0), но даже если бы был захвачен,
        // мы всё равно имеем эксклюзивный доступ к данным через &mut self,
        // но для порядка лучше проверить инвариант.
        // Однако при наличии &mut self других захватов быть не может,
        // поэтому можем просто вернуть ссылку.
        self.data.get_mut()
    }

    /// Разрушает мьютекс и возвращает внутреннее значение.
    ///
    /// # Safety
    /// Вызывающий должен убедиться, что нет активных охранников или ручных захватов.
    pub unsafe fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

/// Охранник, предоставляющий доступ к данным и автоматически освобождающий мьютекс.
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
    thread_id: usize,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.unlock(self.thread_id);
    }
}