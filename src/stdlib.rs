use crate::object::Object;
use std::cell::RefCell;
use std::rc::Rc;

pub fn get_string_method(name: &str) -> Object {
    match name {
        "split" => Object::Native(|args| {
            if let (Some(Object::String(s)), Some(Object::String(sep))) = (args.get(0), args.get(1))
            {
                let parts: Vec<Object> = s
                    .split(sep)
                    .map(|part| Object::String(part.to_string()))
                    .collect();
                Object::Array(Rc::new(RefCell::new(parts)))
            } else {
                Object::Atom("null".to_string())
            }
        }),
        "len" => Object::Native(|args| {
            if let Some(Object::String(s)) = args.get(0) {
                Object::Number(s.len() as f64)
            } else {
                Object::Number(0.0)
            }
        }),
        "trim" => Object::Native(|args| {
            if let Some(Object::String(s)) = args.get(0) {
                Object::String(s.trim().to_string())
            } else {
                Object::Atom("null".to_string())
            }
        }),
        "to_upper" => Object::Native(|args| {
            if let Some(Object::String(s)) = args.get(0) {
                Object::String(s.to_uppercase())
            } else {
                Object::Atom("null".to_string())
            }
        }),
        "to_lower" => Object::Native(|args| {
            if let Some(Object::String(s)) = args.get(0) {
                Object::String(s.to_lowercase())
            } else {
                Object::Atom("null".to_string())
            }
        }),
        _ => Object::Atom("null".to_string()),
    }
}

pub fn get_array_method(name: &str) -> Object {
    match name {
        "push" => Object::Native(|args| {
            if let (Some(Object::Array(arr)), Some(val)) = (args.get(0), args.get(1)) {
                arr.borrow_mut().push(val.clone());
                Object::Number(arr.borrow().len() as f64)
            } else {
                Object::Atom("null".to_string())
            }
        }),
        "pop" => Object::Native(|args| {
            if let Some(Object::Array(arr)) = args.get(0) {
                arr.borrow_mut()
                    .pop()
                    .unwrap_or(Object::Atom("null".to_string()))
            } else {
                Object::Atom("null".to_string())
            }
        }),
        "len" => Object::Native(|args| {
            if let Some(Object::Array(arr)) = args.get(0) {
                Object::Number(arr.borrow().len() as f64)
            } else {
                Object::Number(0.0)
            }
        }),
        "join" => Object::Native(|args| {
            if let (Some(Object::Array(arr)), Some(Object::String(sep))) =
                (args.get(0), args.get(1))
            {
                let items: Vec<String> = arr
                    .borrow()
                    .iter()
                    .map(|item| match item {
                        Object::String(s) => s.clone(),
                        Object::Number(n) => n.to_string(),
                        Object::Atom(a) => a.clone(),
                        other => format!("{:?}", other),
                    })
                    .collect();
                Object::String(items.join(sep))
            } else {
                Object::String("".to_string())
            }
        }),
        _ => Object::Atom("null".to_string()),
    }
}

pub fn get_number_method(name: &str) -> Object {
    match name {
        "to_string" => Object::Native(|args| {
            if let Some(Object::Number(n)) = args.get(0) {
                Object::String(n.to_string())
            } else {
                Object::Atom("null".to_string())
            }
        }),
        "abs" => Object::Native(|args| {
            if let Some(Object::Number(n)) = args.get(0) {
                Object::Number(n.abs())
            } else {
                Object::Number(0.0)
            }
        }),
        "round" => Object::Native(|args| {
            if let Some(Object::Number(n)) = args.get(0) {
                Object::Number(n.round())
            } else {
                Object::Number(0.0)
            }
        }),
        "floor" => Object::Native(|args| {
            if let Some(Object::Number(n)) = args.get(0) {
                Object::Number(n.floor())
            } else {
                Object::Number(0.0)
            }
        }),
        "ceil" => Object::Native(|args| {
            if let Some(Object::Number(n)) = args.get(0) {
                Object::Number(n.ceil())
            } else {
                Object::Number(0.0)
            }
        }),
        _ => Object::Atom("null".to_string()),
    }
}
