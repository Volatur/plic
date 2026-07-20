use crate::napi::alloc::{dealloc_native_function_userdata};
use crate::napi::thread::{NativeFunction, NativeUserdata};
use crate::syntax::parser;
use crate::syntax::parser::{Block, Item};
use crate::utils::alloc::{Array, Boxed};
use crate::vm::heap::{Object, ObjectRef, ObjectRefNN, VMHeap};
use crate::vm::{VMError, VMRef, VMState};
use crate::{napi, ownership_hack, ownership_hack_mut};
use parking_lot::RwLock;
use std::alloc::Layout;
use std::collections::HashMap;
use std::ops::Deref;
use std::ptr::{null_mut, NonNull};
use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};

pub struct VMModuleManager {
    pub modules: RwLock<Vec<Boxed<Module>>>
}

pub struct Module {
    pub path: String,
    pub functions: Array<Function>,
    pub classes: Array<Class>,
    pub objects: Array<Singleton>
}

pub struct Class {
    pub owner: ModuleRef,
    pub name: String,
    pub extends: Array<ClassRef>,
    pub methods: Array<Function>,
    pub layout: Layout
}

pub struct Singleton {
    pub class: Class,
    pub instance: ObjectRef
}

pub enum Function {
    VM {
        name: String,
        params: Array<String>,
        body: Block
    },
    Native {
        name: String,
        params: Array<String>,
        function: NativeFunction,
        userdata: NativeUserdata,
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ModuleRef(pub *mut Module);
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ClassRef(pub NonNull<Class>);

impl VMModuleManager {
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(Vec::new())
        }
    }

    pub fn load_napi_module(vm: VMRef, module: &napi::module::ModuleDef) -> Result<(), VMError> {
        let module = Self::module_parse_napi(module);
        Self::load_module(vm, module)
    }

    pub fn load_ast_module(vm: VMRef, module: &parser::Module) -> Result<(), VMError> {
        let module = Self::module_parse_ast(module);
        Self::load_module(vm, module)
    }

    pub fn find_module(vm: VMRef, path: &str) -> Result<ModuleRef, VMError> {
        if let Some(module) = vm.modules.modules.read().iter().find(|x| x.path == path) {
            Ok(ModuleRef(module.as_raw()))
        } else {
            Err(VMError::ModuleNotFound(path.to_owned()))
        }
    }

    pub fn find_class(vm: VMRef, class: &str) -> Result<ClassRef, VMError> {
        if let Some(class) = Self::try_find_class(vm, class)? {
            Ok(class)
        } else {
            Err(VMError::ClassNotFound(class.to_owned()))
        }
    }

    pub fn try_find_class(vm: VMRef, class: &str) -> Result<Option<ClassRef>, VMError> {
        // SAFETY: Гарантия стандарта.
        let (module, class) = unsafe { class.rsplit_once('/').unwrap_unchecked() };
        Ok(Self::find_module(vm, module)?.classes.iter().find(|x| x.name == class).map(ClassRef::from))
    }

    pub fn find_object(vm: VMRef, object: &str) -> Result<ObjectSmartRefNN, VMError> {
        if let Some(object) = Self::try_find_object(vm, object)? {
            Ok(object)
        } else {
            Err(VMError::ObjectNotFound(object.to_owned()))
        }
    }
    
    pub fn try_find_object(vm: VMRef, object: &str) -> Result<Option<ObjectSmartRefNN>, VMError> {
        // SAFETY: Гарантия стандарта.
        let (module, object) = unsafe { object.rsplit_once('/').unwrap_unchecked() };
        let module = Self::find_module(vm, module)?;
        // SAFETY: Гарантия стандарта.
        let object = module.objects.iter().find(|x| x.class.name == *object);
        let object = object.map(|x| ObjectSmartRefNN::new(unsafe { x.instance.deref().unwrap_unchecked() }));
        Ok(object)
    }

    fn load_module(mut vm: VMRef, module: (String, Array<String>, Array<Function>, Vec<Class>, Vec<Vec<String>>, Vec<Singleton>, Vec<Vec<String>>)) -> Result<(), VMError> {
        let module =
            if vm.modules.modules.read().iter().any(|x| x.path == module.0) {
                Self::module_hotswap(vm, module)?
            } else {
                Self::module_load(vm, module)?
            };
        if let Some(entry) = module.try_find_method("__entry__") {
            vm.call_func(&ObjectSmartRef::null(), entry, &[])?;
        }
        Ok(())
    }

    fn module_hotswap(vm: VMRef, new_module: (String, Array<String>, Array<Function>, Vec<Class>, Vec<Vec<String>>, Vec<Singleton>, Vec<Vec<String>>)) -> Result<ModuleRef, VMError> {
        vm.flags.wait_state(VMState::Running);
        vm.flags.set_state(VMState::HotSwap, false);

        let mut modules = vm.modules.modules.write();
        let old_module = modules.iter().position(|x| x.path == new_module.0);
        // SAFETY: Гарантия вызывающей стороны.
        let old_module = unsafe { old_module.unwrap_unchecked() };
        let old_module = modules.remove(old_module);
        let new_module = Self::module_load(vm, new_module)?;
        drop(modules);

        let mut class_map = HashMap::<ClassRef, ClassRef>::new();
        let mut class_unique = Vec::<ClassRef>::new();

        for old_class in old_module.classes.iter() {
            if let Some(new_class) = new_module.classes.iter().find(|x| x.name == old_class.name) {
                class_map.insert(old_class.into(), new_class.into());
            } else {
                class_unique.push(old_class.into());
            }
        }

        for object in vm.heap.objects.lock().iter_mut() {
            // SAFETY: Все указатели в heap валидные.
            if let Some(new_class) = class_map.get(&object.class) {
                object.class = new_class.clone();
            } else if class_unique.contains(&object.class) {
                return Err(VMError::ClassNotFound(object.class.name.to_owned()));
            }
        }

        vm.flags.set_state(VMState::Running, true);
        Ok(new_module)
    }

    fn module_load(mut vm: VMRef, module: (String, Array<String>, Array<Function>, Vec<Class>, Vec<Vec<String>>, Vec<Singleton>, Vec<Vec<String>>)) -> Result<ModuleRef, VMError> {
        let (path, _uses, functions, classes, class_extends, objects, object_extends) = module;

        let mut module = Boxed::new(
            Module {
                path: path.clone(),
                functions,
                classes: Array::from(classes),
                objects: Array::from(objects),
            }
        );

        let ref_module = &mut module;
        let classes = &mut ref_module.classes;
        let classes = ownership_hack_mut(classes);
        let objects = &mut ref_module.objects;
        let objects = ownership_hack_mut(objects);
        let ref_module = ModuleRef(module.as_raw());

        vm.modules.modules.write().push(module);

        // todo: загрузка зависимостей из uses

        for i in 0..classes.len() {
            // SAFETY: Проверка по длине выше.
            let class = unsafe { classes.get_unchecked_mut(i) };
            // SAFETY: Длина списка равна длине classes.
            let extends = unsafe { class_extends.get_unchecked(i) };
            Self::module_load_init_class(vm, class, ref_module, extends)?;
        }

        for i in 0..objects.len() {
            // SAFETY: Проверка по длине выше.
            let object = unsafe { objects.get_unchecked_mut(i) };
            // SAFETY: Длина списка равна длине classes.
            let extends = unsafe { object_extends.get_unchecked(i) };
            Self::module_load_init_class(vm, &mut object.class, ref_module, extends)?;
        }

        for i in 0..objects.len() {
            // SAFETY: Проверка по длине выше.
            let object = unsafe { objects.get_unchecked_mut(i) };
            let class = &mut object.class;
            let class = ClassRef::from(class);
            let instance = VMHeap::alloc(vm, class)?;
            let instance = vm.call_obj(&instance, "__init__", &[])?.deref()?;
            object.instance = instance.as_raw().into();
        }

        Ok(ref_module)
    }

    fn module_load_init_class(vm: VMRef, class: &mut Class, owner: ModuleRef, extends: &Vec<String>) -> Result<(), VMError> {
        let mut new_extends = Vec::new();
        for extend in extends {
            new_extends.push(Self::find_class(vm, extend)?);
        }
        class.extends = Array::from(new_extends);
        class.owner = owner;
        Ok(())
    }


    // Внимание! Не выполняет инициализацию extends у class и instance у object
    fn module_parse_napi(module: &napi::module::ModuleDef) -> (String, Array<String>, Array<Function>, Vec<Class>, Vec<Vec<String>>, Vec<Singleton>, Vec<Vec<String>>) {
        let path = module.path.clone();
        let uses = Array::from(&module.uses);
        let functions = module.functions
            .iter()
            .map(Self::module_parse_napi_function)
            .collect();
        let mut class_extends = Vec::new();
        let classes = module.classes
            .iter()
            .map(|class| {
                let (class, extends) = Self::module_parse_napi_class(class);
                class_extends.push(extends);
                class
            })
            .collect();
        let mut object_extends = Vec::new();
        let objects = module.objects
            .iter()
            .map(|object| {
                let (class, extends) = Self::module_parse_napi_class(object);
                object_extends.push(extends);
                // SAFETY: Ожидается дальнейшая инициализация перед использованием.
                Singleton { class, instance: ObjectRef::null() }
            })
            .collect();
        (
            path,
            uses,
            functions,
            classes,
            class_extends,
            objects,
            object_extends
        )
    }

    // Внимание! Не выполняет инициализацию owner / extends у class
    fn module_parse_napi_class(class: &napi::module::ClassDef) -> (Class, Vec<String>) {
        (
            Class {
                owner: ModuleRef(null_mut()),
                name: class.name.clone(),
                extends: Array::empty(),
                methods: class.methods.iter().map(Self::module_parse_napi_function).collect(),
                layout: unsafe { Layout::array::<u8>(size_of::<Object>() + class.allocation).unwrap_unchecked() }
            },
            class.extends.clone(),
        )
    }

    fn module_parse_napi_function(function: &napi::module::FunctionDef) -> Function {
        match &function.body {
            napi::module::FunctionBodyDef::AST(body) => {
                Function::VM {
                    name: function.name.to_owned(),
                    params: Array::from(&function.params),
                    body: body.clone()
                }
            },
            napi::module::FunctionBodyDef::Native(native) => {
                Function::Native {
                    name: function.name.to_owned(),
                    params: Array::from(&function.params),
                    function: *native,
                    userdata: null_mut()
                }
            }
        }
    }

    // Внимание! Не выполняет инициализацию extends у class и instance у object
    fn module_parse_ast(module: &parser::Module) -> (String, Array<String>, Array<Function>, Vec<Class>, Vec<Vec<String>>, Vec<Singleton>, Vec<Vec<String>>) {
        let path = module.path.clone();
        let uses = Array::from(&module.uses);
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut class_extends = Vec::new();
        let mut objects = Vec::new();
        let mut object_extends = Vec::new();
        module.items.iter().for_each(|item| {
            match item {
                Item::Function(function) => {
                    functions.push(Function::VM {
                        name: function.name.clone(),
                        params: Array::from(&function.params),
                        body: function.body.clone()
                    })
                },
                Item::Class(class) => {
                    let (class, extends) = Self::module_parse_ast_class(class);
                    classes.push(class);
                    class_extends.push(extends);
                },
                Item::Object(class) => {
                    let (class, extends) = Self::module_parse_ast_class(class);
                    // SAFETY: Ожидается дальнейшая инициализация перед использованием.
                    objects.push(Singleton { class, instance: ObjectRef::null() });
                    object_extends.push(extends);
                }
            }
        });
        (
            path,
            uses,
            Array::from(functions),
            classes,
            class_extends,
            objects,
            object_extends
        )
    }

    // Внимание! Не выполняет инициализацию owner / extends у class
    fn module_parse_ast_class(class: &parser::Class) -> (Class, Vec<String>) {
        (
            Class {
                owner: ModuleRef(null_mut()),
                name: class.name.clone(),
                extends: Array::empty(),
                methods: class.methods
                    .iter()
                    .map(|function|
                        Function::VM {
                            name: function.name.clone(),
                            params: Array::from(&function.params),
                            body: function.body.clone()
                        }
                    )
                    .collect(),
                layout: Layout::new::<Object>()
            },
            class.extends.clone(),
        )
    }
}

impl Module {
    pub fn find_method<'a, 'b>(&'a self, name: &str) -> Result<&'b Function, VMError> {
        if let Some(method) = self.try_find_method(name) {
            Ok(ownership_hack(method))
        } else {
            Err(VMError::MethodNotFound(name.to_owned()))
        }
    }

    pub fn try_find_method<'a, 'b>(&'a self, name: &str) -> Option<&'b Function> {
        self.functions.iter().find(|x| x.name() == name).map(|x| ownership_hack(x))
    }
}

impl Class {
    pub fn find_method<'a, 'b>(&'a self, name: &str) -> Result<&'b Function, VMError> {
        if let Some(method) = self.try_find_method(name) {
            Ok(ownership_hack(method))
        } else {
            Err(VMError::MethodNotFound(name.to_owned()))
        }
    }

    pub fn try_find_method<'a, 'b>(&'a self, name: &str) -> Option<&'b Function> {
        if let Some(find) = self.methods.iter().find(|x| x.name() == name) {
            return Some(ownership_hack(find))
        }

        for class in self.extends.iter() {
            if let Some(find) = class.try_find_method(name) {
                return Some(find)
            }
        }

        None
    }
}

impl Singleton {
    pub fn instance(&self) -> ObjectRefNN {
        // SAFETY: Гарантия стандарта.
        unsafe { ObjectRefNN::new_unchecked(self.instance.0) }
    }
}

impl Function {
    pub fn name(&self) -> &String {
        match self {
            Function::VM { name, .. } => name,
            Function::Native { name, .. } => name
        }
    }

    pub fn params(&self) -> &Array<String> {
        match self {
            Function::VM { params, .. } => params,
            Function::Native { params, .. } => params
        }
    }

    pub fn is_native(&self) -> bool {
        match self {
            Function::VM { .. } => false,
            Function::Native { .. } => true,
        }
    }
}

impl Drop for Function {
    fn drop(&mut self) {
        if let Function::Native { userdata, .. } = self && !userdata.is_null() {
            // SAFETY: Гарантия стандарта.
            unsafe {
                dealloc_native_function_userdata(*userdata)
            }
        }
    }
}

impl Deref for ModuleRef {
    type Target = Module;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Гарантия структуры.
        unsafe { &*self.0 }
    }
}

impl From<&Class> for ClassRef {
    fn from(class: &Class) -> Self {
        Self(NonNull::from(class))
    }
}

impl From<&mut Class> for ClassRef {
    fn from(class: &mut Class) -> Self {
        Self(NonNull::from(class))
    }
}

impl Deref for ClassRef {
    type Target = Class;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Гарантия структуры.
        unsafe { self.0.as_ref() }
    }
}
