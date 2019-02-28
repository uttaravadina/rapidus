#![macro_use]

use bytecode_gen::ByteCode;
use rustc_hash::FxHashMap;
use vm::error::RuntimeError;
use vm::value::Value2;
use vm::vm::VMResult;

pub type LexicalEnvironmentRef = *mut LexicalEnvironment;

#[derive(Debug, Clone)]
pub struct Frame {
    pub execution_context: ExecutionContext,
    pub pc: usize,
    pub sp: usize,
    pub bytecode: ByteCode,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub variable_environment: LexicalEnvironmentRef,
    pub lexical_environment: LexicalEnvironmentRef,
    pub saved_lexical_environment: Vec<LexicalEnvironmentRef>,
}

#[derive(Debug, Clone)]
pub struct LexicalEnvironment {
    pub record: EnvironmentRecord,
    pub outer: Option<LexicalEnvironmentRef>,
}

#[derive(Debug, Clone)]
pub enum EnvironmentRecord {
    Declarative(FxHashMap<String, Value2>),
    Object(Value2),
    Global(FxHashMap<String, Value2>),
}

impl Frame {
    pub fn new(execution_context: ExecutionContext, bytecode: ByteCode) -> Self {
        Frame {
            execution_context,
            pc: 0,
            sp: 0,
            bytecode,
        }
    }

    pub fn lex_env(&mut self) -> &mut LexicalEnvironment {
        unsafe { &mut *self.execution_context.lexical_environment }
    }
}

impl ExecutionContext {
    pub fn new(env: *mut LexicalEnvironment) -> Self {
        ExecutionContext {
            variable_environment: env,
            lexical_environment: env,
            saved_lexical_environment: vec![],
        }
    }
}

#[macro_export]
macro_rules! make_global_env {
    ($($property_name:ident : $val:expr),*) => { {
        let mut record = FxHashMap::default();
        $( record.insert((stringify!($property_name)).to_string(), $val); )*
        record
    } };
}

impl LexicalEnvironment {
    pub fn new_declarative(outer: Option<*mut LexicalEnvironment>) -> Self {
        LexicalEnvironment {
            record: EnvironmentRecord::Declarative(FxHashMap::default()),
            outer,
        }
    }

    pub fn new_object(object: Value2, outer: Option<*mut LexicalEnvironment>) -> Self {
        LexicalEnvironment {
            record: EnvironmentRecord::Object(object),
            outer,
        }
    }

    pub fn new_global() -> Self {
        LexicalEnvironment {
            record: EnvironmentRecord::Global(FxHashMap::default()),
            outer: None,
        }
    }

    pub fn new_global_initialized() -> Self {
        LexicalEnvironment {
            // TODO: 'log' for the time being
            record: EnvironmentRecord::Global(make_global_env!(log: Value2::Number(1.0))),
            outer: None,
        }
    }

    pub fn get_value(&self, name: &String) -> Result<Value2, RuntimeError> {
        match self.record {
            EnvironmentRecord::Declarative(ref record) | EnvironmentRecord::Global(ref record) => {
                match record.get(name) {
                    Some(binding) if binding == &Value2::uninitialized() => {
                        return Err(RuntimeError::Reference(format!(
                            "'{}' is not defined",
                            name
                        )));
                    }
                    Some(binding) => return Ok(*binding),
                    None => {}
                }
            }
            EnvironmentRecord::Object(_) => unimplemented!(),
        };

        if let Some(outer) = self.get_outer() {
            outer.get_value(name)
        } else {
            Err(RuntimeError::Reference(format!(
                "'{}' is not defined",
                name
            )))
        }
    }

    pub fn set_value(&mut self, name: String, val: Value2) -> VMResult {
        match self.record {
            EnvironmentRecord::Declarative(ref mut record) => match record.get_mut(&name) {
                Some(binding) => {
                    *binding = val;
                    return Ok(());
                }
                None => {}
            },
            EnvironmentRecord::Global(ref mut record) => {
                record.insert(name, val);
                return Ok(());
            }
            EnvironmentRecord::Object(_) => unimplemented!(),
        };

        if let Some(outer) = self.get_outer() {
            outer.set_value(name, val)?
        } else {
            return Err(RuntimeError::Reference(format!(
                "Assignment to undeclared identifier '{}'",
                name
            )));
        }

        Ok(())
    }

    pub fn get_outer(&self) -> Option<&mut LexicalEnvironment> {
        self.outer.and_then(|outer| Some(unsafe { &mut *outer }))
    }
}