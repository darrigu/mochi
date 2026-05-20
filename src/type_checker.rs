use crate::ast::{Expression, TypeAnn};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Type {
    Var(usize),
    Number,
    String,
    Atom,
    Array(Rc<RefCell<Type>>),
    Hash(Rc<RefCell<HashMap<String, Type>>>),
    Function { params: Vec<Type>, ret: Box<Type> },
    Any,
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Type::Var(id1), Type::Var(id2)) => id1 == id2,
            (Type::Number, Type::Number) => true,
            (Type::String, Type::String) => true,
            (Type::Atom, Type::Atom) => true,
            (Type::Any, Type::Any) => true,
            (Type::Array(t1), Type::Array(t2)) => t1.borrow().eq(&*t2.borrow()),
            (Type::Hash(h1), Type::Hash(h2)) => h1.borrow().eq(&*h2.borrow()),
            (
                Type::Function {
                    params: p1,
                    ret: r1,
                },
                Type::Function {
                    params: p2,
                    ret: r2,
                },
            ) => p1 == p2 && r1 == r2,
            _ => false,
        }
    }
}
impl Eq for Type {}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Var(id) => write!(f, "'t{}", id),
            Type::Number => write!(f, "Number"),
            Type::String => write!(f, "String"),
            Type::Atom => write!(f, "Atom"),
            Type::Any => write!(f, "Any"),
            Type::Array(inner) => write!(f, "Array<{}>", inner.borrow()),
            Type::Hash(fields) => {
                let map = fields.borrow();
                let pairs: Vec<String> = map.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{ {} }}", pairs.join(", "))
            }
            Type::Function { params, ret } => {
                let param_strs: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "({}) -> {}", param_strs.join(", "), ret)
            }
        }
    }
}

#[derive(Clone)]
pub struct TypeEnv {
    pub store: HashMap<String, (Type, bool)>,
    pub outer: Option<Rc<RefCell<TypeEnv>>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut env = Self {
            store: HashMap::new(),
            outer: None,
        };
        env.store.insert("print".to_string(), (Type::Any, true));
        env
    }

    pub fn new_enclosed(outer: Rc<RefCell<TypeEnv>>) -> Self {
        Self {
            store: HashMap::new(),
            outer: Some(outer),
        }
    }

    pub fn define(&mut self, name: String, ty: Type, is_const: bool) {
        self.store.insert(name, (ty, is_const));
    }

    pub fn resolve(&self, name: &str) -> Option<(Type, bool)> {
        if let Some(entry) = self.store.get(name) {
            Some(entry.clone())
        } else if let Some(outer) = &self.outer {
            outer.borrow().resolve(name)
        } else {
            None
        }
    }
}

pub struct TypeChecker {
    next_var_id: usize,
    substitutions: HashMap<usize, Type>,
    current_return_type: Option<Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            next_var_id: 0,
            substitutions: HashMap::new(),
            current_return_type: None,
        }
    }

    fn new_var(&mut self) -> Type {
        let id = self.next_var_id;
        self.next_var_id += 1;
        Type::Var(id)
    }

    pub fn find(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(id) => {
                if let Some(substituted) = self.substitutions.get(id) {
                    self.find(substituted)
                } else {
                    ty.clone()
                }
            }
            Type::Array(inner) => {
                let resolved_inner = self.find(&*inner.borrow());
                Type::Array(Rc::new(RefCell::new(resolved_inner)))
            }
            Type::Hash(fields) => {
                let mut resolved_fields = HashMap::new();
                for (k, v) in fields.borrow().iter() {
                    resolved_fields.insert(k.clone(), self.find(v));
                }
                Type::Hash(Rc::new(RefCell::new(resolved_fields)))
            }
            Type::Function { params, ret } => {
                let resolved_params = params.iter().map(|p| self.find(p)).collect();
                let resolved_ret = Box::new(self.find(ret));
                Type::Function {
                    params: resolved_params,
                    ret: resolved_ret,
                }
            }
            _ => ty.clone(),
        }
    }

    pub fn unify(&mut self, t1: &Type, t2: &Type) -> Result<(), String> {
        let t1 = self.find(t1);
        let t2 = self.find(t2);

        if t1 == t2 {
            return Ok(());
        }

        match (t1, t2) {
            (Type::Var(id1), t2) => {
                if self.occurs_in(id1, &t2) {
                    return Err("Infinite type detected (occurs check failed)".to_string());
                }
                self.substitutions.insert(id1, t2);
                Ok(())
            }
            (t1, Type::Var(id2)) => {
                if self.occurs_in(id2, &t1) {
                    return Err("Infinite type detected (occurs check failed)".to_string());
                }
                self.substitutions.insert(id2, t1);
                Ok(())
            }
            (Type::Array(inner1), Type::Array(inner2)) => {
                self.unify(&*inner1.borrow(), &*inner2.borrow())
            }
            (Type::Hash(fields1), Type::Hash(fields2)) => {
                let keys1: HashSet<String> = fields1.borrow().keys().cloned().collect();
                let keys2: HashSet<String> = fields2.borrow().keys().cloned().collect();

                let is_subset_1_in_2 = keys1.is_subset(&keys2);
                let is_subset_2_in_1 = keys2.is_subset(&keys1);

                if !is_subset_1_in_2 && !is_subset_2_in_1 {
                    return Err(format!(
                        "Record type mismatch: incompatible fields. Got {:?}, expected {:?}",
                        keys1, keys2
                    ));
                }

                for k in keys1.intersection(&keys2) {
                    let v1 = fields1.borrow().get(k).unwrap().clone();
                    let v2 = fields2.borrow().get(k).unwrap().clone();
                    self.unify(&v1, &v2)?;
                }

                if is_subset_1_in_2 {
                    for k in keys2.difference(&keys1) {
                        let v2 = fields2.borrow().get(k).unwrap().clone();
                        fields1.borrow_mut().insert(k.clone(), v2);
                    }
                } else if is_subset_2_in_1 {
                    for k in keys1.difference(&keys2) {
                        let v1 = fields1.borrow().get(k).unwrap().clone();
                        fields2.borrow_mut().insert(k.clone(), v1);
                    }
                }

                Ok(())
            }
            (
                Type::Function {
                    params: p1,
                    ret: r1,
                },
                Type::Function {
                    params: p2,
                    ret: r2,
                },
            ) => {
                if p1.len() != p2.len() {
                    return Err(format!(
                        "Function arity mismatch: expected {} arguments, got {}",
                        p1.len(),
                        p2.len()
                    ));
                }
                for (a1, a2) in p1.iter().zip(p2.iter()) {
                    self.unify(a1, a2)?;
                }
                self.unify(&r1, &r2)
            }
            (Type::Any, _) | (_, Type::Any) => Ok(()),
            (a, b) => Err(format!("Type mismatch: cannot unify '{}' with '{}'", a, b)),
        }
    }

    fn occurs_in(&self, id: usize, ty: &Type) -> bool {
        let ty = self.find(ty);
        match ty {
            Type::Var(v) => v == id,
            Type::Array(inner) => self.occurs_in(id, &*inner.borrow()),
            Type::Hash(fields) => fields.borrow().values().any(|v| self.occurs_in(id, v)),
            Type::Function { params, ret } => {
                params.iter().any(|p| self.occurs_in(id, p)) || self.occurs_in(id, &ret)
            }
            _ => false,
        }
    }

    fn map_type_ann(&mut self, ann: &TypeAnn) -> Type {
        match ann {
            TypeAnn::Number => Type::Number,
            TypeAnn::String => Type::String,
            TypeAnn::Atom => Type::Atom,
            TypeAnn::Any => Type::Any,
            TypeAnn::Array(inner) => {
                let mapped_inner = self.map_type_ann(inner);
                Type::Array(Rc::new(RefCell::new(mapped_inner)))
            }
            TypeAnn::Hash(fields) => {
                let mut mapped_fields = HashMap::new();
                for (k, v) in fields {
                    mapped_fields.insert(k.clone(), self.map_type_ann(v));
                }
                Type::Hash(Rc::new(RefCell::new(mapped_fields)))
            }
            TypeAnn::Function { params, ret } => {
                let mapped_params = params.iter().map(|p| self.map_type_ann(p)).collect();
                let mapped_ret = Box::new(self.map_type_ann(ret));
                Type::Function {
                    params: mapped_params,
                    ret: mapped_ret,
                }
            }
        }
    }

    pub fn check_expected(
        &mut self,
        expr: &Expression,
        env: &Rc<RefCell<TypeEnv>>,
        expected: &Type,
    ) -> Result<Type, String> {
        let expected = self.find(expected);
        match (expr, &expected) {
            (&Expression::Array(ref elements), &Type::Array(ref expected_elem_ty)) => {
                let expected_elem_ty_inner = expected_elem_ty.borrow().clone();
                for el in elements {
                    let el_ty = self.check(el, env)?;
                    self.unify(&el_ty, &expected_elem_ty_inner)?;
                }
                Ok(Type::Array(expected_elem_ty.clone()))
            }
            (&Expression::Hash(ref pairs), &Type::Hash(ref expected_fields)) => {
                let actual_keys: HashSet<String> = pairs.iter().map(|(k, _)| k.clone()).collect();
                let expected_keys: HashSet<String> =
                    expected_fields.borrow().keys().cloned().collect();

                if !expected_keys.is_subset(&actual_keys) {
                    let missing: Vec<String> =
                        expected_keys.difference(&actual_keys).cloned().collect();
                    return Err(format!(
                        "Record type mismatch: missing required fields {:?}",
                        missing
                    ));
                }

                let mut actual_fields = HashMap::new();
                for (key, val) in pairs {
                    let val_ty = if let Some(expected_val_ty) = expected_fields.borrow().get(key) {
                        self.check_expected(val, env, expected_val_ty)?
                    } else {
                        self.check(val, env)?
                    };
                    actual_fields.insert(key.clone(), val_ty);
                }

                let actual_hash_ty = Type::Hash(Rc::new(RefCell::new(actual_fields)));
                self.unify(&actual_hash_ty, &expected)?;

                Ok(actual_hash_ty)
            }
            _ => {
                let val_ty = self.check(expr, env)?;
                self.unify(&val_ty, &expected)?;
                Ok(val_ty)
            }
        }
    }

    pub fn check(&mut self, expr: &Expression, env: &Rc<RefCell<TypeEnv>>) -> Result<Type, String> {
        match expr {
            Expression::Identifier(name) => {
                if let Some((ty, _)) = env.borrow().resolve(name) {
                    Ok(ty)
                } else {
                    Err(format!("Undefined variable: '{}'", name))
                }
            }
            Expression::Number(_) => Ok(Type::Number),
            Expression::StringLiteral(_) => Ok(Type::String),
            Expression::Atom(_) => Ok(Type::Atom),
            Expression::Array(elements) => {
                let elem_ty = self.new_var();
                let mut failed_homogeneous = false;

                for el in elements {
                    let el_ty = self.check(el, env)?;
                    if self.unify(&elem_ty, &el_ty).is_err() {
                        failed_homogeneous = true;
                    }
                }

                if failed_homogeneous {
                    Ok(Type::Array(Rc::new(RefCell::new(Type::Any))))
                } else {
                    Ok(Type::Array(Rc::new(RefCell::new(self.find(&elem_ty)))))
                }
            }
            Expression::Hash(pairs) => {
                let mut fields = HashMap::new();
                for (key, val) in pairs {
                    let val_ty = self.check(val, env)?;
                    fields.insert(key.clone(), val_ty);
                }
                Ok(Type::Hash(Rc::new(RefCell::new(fields))))
            }
            Expression::Prefix { operator, right } => {
                let right_ty = self.check(right, env)?;
                match operator.as_str() {
                    "-" => {
                        self.unify(&right_ty, &Type::Number)?;
                        Ok(Type::Number)
                    }
                    "!" => Ok(Type::Atom),
                    _ => Err(format!("Unknown prefix operator: '{}'", operator)),
                }
            }
            Expression::Infix {
                left,
                operator,
                right,
            } => {
                let left_ty = self.check(left, env)?;
                let right_ty = self.check(right, env)?;

                match operator.as_str() {
                    "+" => {
                        self.unify(&left_ty, &right_ty)?;
                        let resolved = self.find(&left_ty);
                        match resolved {
                            Type::Number | Type::String | Type::Var(_) | Type::Any => Ok(left_ty),
                            other => Err(format!(
                                "Operator '+' is not supported for type '{}'",
                                other
                            )),
                        }
                    }
                    "-" | "*" | "/" => {
                        self.unify(&left_ty, &Type::Number)?;
                        self.unify(&right_ty, &Type::Number)?;
                        Ok(Type::Number)
                    }
                    "==" | "!=" => {
                        self.unify(&left_ty, &right_ty)?;
                        Ok(Type::Atom)
                    }
                    ">" | "<" => {
                        self.unify(&left_ty, &Type::Number)?;
                        self.unify(&right_ty, &Type::Number)?;
                        Ok(Type::Atom)
                    }
                    _ => Err(format!("Unknown infix operator: '{}'", operator)),
                }
            }
            Expression::If {
                condition,
                consequence,
                alternative,
            } => {
                let _cond_ty = self.check(condition, env)?;
                let cons_ty = self.check(consequence, env)?;

                if let Some(alt) = alternative {
                    let alt_ty = self.check(alt, env)?;
                    self.unify(&cons_ty, &alt_ty)?;
                    Ok(cons_ty)
                } else {
                    Ok(Type::Any)
                }
            }
            Expression::Loop { body } => {
                let _body_ty = self.check(body, env)?;
                Ok(Type::Atom)
            }
            Expression::While { condition, body } => {
                let _cond_ty = self.check(condition, env)?;
                let _body_ty = self.check(body, env)?;
                Ok(Type::Atom)
            }
            Expression::For {
                element,
                iterable,
                body,
            } => {
                let iter_ty = self.check(iterable, env)?;
                let resolved_iter = self.find(&iter_ty);

                let loop_env = Rc::new(RefCell::new(TypeEnv::new_enclosed(env.clone())));

                let elem_ty = match resolved_iter {
                    Type::Array(inner) => inner.borrow().clone(),
                    Type::Var(_) => {
                        let inner = self.new_var();
                        self.unify(&iter_ty, &Type::Array(Rc::new(RefCell::new(inner.clone()))))?;
                        inner
                    }
                    _ => Type::Any,
                };

                loop_env
                    .borrow_mut()
                    .define(element.clone(), elem_ty, false);
                let _body_ty = self.check(body, &loop_env)?;
                Ok(Type::Atom)
            }
            Expression::ForHash {
                key,
                value,
                iterable,
                body,
            } => {
                let iter_ty = self.check(iterable, env)?;
                let resolved_iter = self.find(&iter_ty);

                let loop_env = Rc::new(RefCell::new(TypeEnv::new_enclosed(env.clone())));

                match resolved_iter {
                    Type::Hash(_) => {}
                    Type::Var(_) => {
                        let hash_ty =
                            Type::Hash(Rc::new(RefCell::new(std::collections::HashMap::new())));
                        self.unify(&iter_ty, &hash_ty)?;
                    }
                    _ => {}
                };

                loop_env
                    .borrow_mut()
                    .define(key.clone(), Type::String, false);
                loop_env
                    .borrow_mut()
                    .define(value.clone(), Type::Any, false);
                let _body_ty = self.check(body, &loop_env)?;
                Ok(Type::Atom)
            }
            Expression::Block(expressions) => {
                let block_env = Rc::new(RefCell::new(TypeEnv::new_enclosed(env.clone())));
                if expressions.is_empty() {
                    return Ok(Type::Atom);
                }
                let mut last_ty = Type::Atom;
                for expr in expressions {
                    last_ty = self.check(expr, &block_env)?;
                }
                Ok(last_ty)
            }
            Expression::Let {
                name,
                type_ann,
                value,
            } => {
                let val_ty = if let Some(ann) = type_ann {
                    let expected_ty = self.map_type_ann(ann);
                    if let Expression::Function { .. } = &**value {
                        env.borrow_mut()
                            .define(name.clone(), expected_ty.clone(), false);
                    }
                    let _checked_ty = self.check_expected(value, env, &expected_ty)?;
                    env.borrow_mut()
                        .define(name.clone(), expected_ty.clone(), false);
                    expected_ty
                } else {
                    let val_ty = if let Expression::Function { .. } = &**value {
                        let placeholder = self.new_var();
                        env.borrow_mut()
                            .define(name.clone(), placeholder.clone(), false);
                        let actual_ty = self.check(value, env)?;
                        self.unify(&placeholder, &actual_ty)?;
                        actual_ty
                    } else {
                        self.check(value, env)?
                    };
                    env.borrow_mut().define(name.clone(), val_ty.clone(), false);
                    val_ty
                };
                Ok(val_ty)
            }
            Expression::Const {
                name,
                type_ann,
                value,
            } => {
                let val_ty = if let Some(ann) = type_ann {
                    let expected_ty = self.map_type_ann(ann);
                    if let Expression::Function { .. } = &**value {
                        env.borrow_mut()
                            .define(name.clone(), expected_ty.clone(), true);
                    }
                    let _checked_ty = self.check_expected(value, env, &expected_ty)?;
                    env.borrow_mut()
                        .define(name.clone(), expected_ty.clone(), true);
                    expected_ty
                } else {
                    let val_ty = if let Expression::Function { .. } = &**value {
                        let placeholder = self.new_var();
                        env.borrow_mut()
                            .define(name.clone(), placeholder.clone(), true);
                        let actual_ty = self.check(value, env)?;
                        self.unify(&placeholder, &actual_ty)?;
                        actual_ty
                    } else {
                        self.check(value, env)?
                    };
                    env.borrow_mut().define(name.clone(), val_ty.clone(), true);
                    val_ty
                };
                Ok(val_ty)
            }
            Expression::Assign { name, value } => {
                let resolved = env.borrow().resolve(name);
                if let Some((existing_ty, is_const)) = resolved {
                    if is_const {
                        return Err(format!("Cannot reassign constant '{}'", name));
                    }
                    let val_ty = self.check(value, env)?;
                    self.unify(&existing_ty, &val_ty)?;
                    Ok(val_ty)
                } else {
                    Err(format!("Undefined variable: '{}'", name))
                }
            }
            Expression::Return(expr) => {
                let expr_ty = self.check(expr, env)?;
                if let Some(expected_ty) = self.current_return_type.clone() {
                    self.unify(&expr_ty, &expected_ty)?;
                } else {
                    return Err("Return statement outside function context".to_string());
                }
                Ok(expr_ty)
            }
            Expression::Function {
                parameters,
                return_type,
                body,
            } => {
                let fn_env = Rc::new(RefCell::new(TypeEnv::new_enclosed(env.clone())));
                let mut param_types = vec![];

                for (param, type_ann) in parameters {
                    let p_ty = if let Some(ann) = type_ann {
                        self.map_type_ann(ann)
                    } else {
                        self.new_var()
                    };
                    fn_env
                        .borrow_mut()
                        .define(param.clone(), p_ty.clone(), false);
                    param_types.push(p_ty);
                }

                let prev_ret = self.current_return_type.clone();
                let expected_ret = if let Some(ann) = return_type {
                    self.map_type_ann(ann)
                } else {
                    self.new_var()
                };
                self.current_return_type = Some(expected_ret.clone());

                let mut body_ty = Type::Atom;
                for expr in body {
                    body_ty = self.check(expr, &fn_env)?;
                }

                self.unify(&body_ty, &expected_ret)?;
                let final_ret = self.find(&expected_ret);
                self.current_return_type = prev_ret;

                let final_params = param_types.iter().map(|p| self.find(p)).collect();

                Ok(Type::Function {
                    params: final_params,
                    ret: Box::new(final_ret),
                })
            }
            Expression::Call {
                function,
                arguments,
            } => {
                let fn_ty = self.check(function, env)?;
                let resolved_fn = self.find(&fn_ty);

                match resolved_fn {
                    Type::Function { params, ret } => {
                        if params.len() != arguments.len() {
                            return Err(format!(
                                "Function arity mismatch: expected {} arguments, got {}",
                                params.len(),
                                arguments.len()
                            ));
                        }
                        let mut arg_types = vec![];
                        for (arg, param_ty) in arguments.iter().zip(params.iter()) {
                            arg_types.push(self.check_expected(arg, env, param_ty)?);
                        }
                        Ok(*ret)
                    }
                    Type::Any => {
                        let mut arg_types = vec![];
                        for arg in arguments {
                            arg_types.push(self.check(arg, env)?);
                        }
                        Ok(Type::Any)
                    }
                    _ => {
                        let mut arg_types = vec![];
                        for arg in arguments {
                            arg_types.push(self.check(arg, env)?);
                        }
                        let ret_ty = self.new_var();
                        self.unify(
                            &fn_ty,
                            &Type::Function {
                                params: arg_types,
                                ret: Box::new(ret_ty.clone()),
                            },
                        )?;
                        Ok(ret_ty)
                    }
                }
            }
            Expression::MethodCall {
                left,
                method,
                arguments,
            } => {
                let left_ty = self.check(left, env)?;
                let mut checked_args = vec![left_ty.clone()];

                for arg in arguments {
                    checked_args.push(self.check(arg, env)?);
                }

                let ret_ty = self.new_var();
                let resolved_left = self.find(&left_ty);

                match resolved_left {
                    Type::Hash(fields) => {
                        if let Some(method_ty) = fields.borrow().get(method).cloned() {
                            let resolved_method = self.find(&method_ty);
                            match resolved_method {
                                Type::Function { params, ret } => {
                                    if params.len() != arguments.len() + 1 {
                                        return Err(format!(
                                            "Function arity mismatch: expected {} arguments, got {}",
                                            params.len() - 1,
                                            arguments.len()
                                        ));
                                    }
                                    self.unify(&left_ty, &params[0])?;

                                    for (arg, param_ty) in
                                        arguments.iter().zip(params.iter().skip(1))
                                    {
                                        self.check_expected(arg, env, param_ty)?;
                                    }
                                    Ok(*ret)
                                }
                                _ => {
                                    let mut checked_args = vec![left_ty.clone()];
                                    for arg in arguments {
                                        checked_args.push(self.check(arg, env)?);
                                    }
                                    let ret_ty = self.new_var();
                                    self.unify(
                                        &method_ty,
                                        &Type::Function {
                                            params: checked_args,
                                            ret: Box::new(ret_ty.clone()),
                                        },
                                    )?;
                                    Ok(ret_ty)
                                }
                            }
                        } else {
                            let mut checked_args = vec![left_ty.clone()];
                            let mut method_args = checked_args.clone();
                            method_args[0] = Type::Any;
                            for arg in arguments {
                                let arg_ty = self.check(arg, env)?;
                                checked_args.push(arg_ty.clone());
                                method_args.push(arg_ty);
                            }
                            let ret_ty = self.new_var();
                            let method_ty = Type::Function {
                                params: method_args,
                                ret: Box::new(ret_ty.clone()),
                            };
                            fields.borrow_mut().insert(method.clone(), method_ty);
                            Ok(ret_ty)
                        }
                    }
                    Type::Any => Ok(Type::Any),
                    Type::Var(_) => {
                        let mut fields = HashMap::new();
                        let mut method_args = checked_args.clone();
                        method_args[0] = Type::Any;

                        let method_ty = Type::Function {
                            params: method_args,
                            ret: Box::new(ret_ty.clone()),
                        };
                        fields.insert(method.clone(), method_ty);
                        let hash_ty = Type::Hash(Rc::new(RefCell::new(fields)));
                        self.unify(&left_ty, &hash_ty)?;
                        Ok(ret_ty)
                    }
                    other => Err(format!(
                        "Method call failed: receiver is of type '{}', which is not an object",
                        other
                    )),
                }
            }
            Expression::Index { left, index } => {
                let left_ty = self.check(left, env)?;
                let index_ty = self.check(index, env)?;

                let resolved_left = self.find(&left_ty);
                match resolved_left {
                    Type::Array(elem_ty) => {
                        self.unify(&index_ty, &Type::Number)?;
                        Ok(elem_ty.borrow().clone())
                    }
                    Type::Hash(fields) => {
                        let static_key = match &**index {
                            Expression::StringLiteral(s) => Some(s.clone()),
                            Expression::Atom(s) => Some(s.clone()),
                            _ => None,
                        };
                        if let Some(key) = static_key {
                            if let Some(ty) = fields.borrow().get(&key) {
                                Ok(ty.clone())
                            } else {
                                let new_prop_ty = self.new_var();
                                fields.borrow_mut().insert(key, new_prop_ty.clone());
                                Ok(new_prop_ty)
                            }
                        } else {
                            self.unify(&index_ty, &Type::String)?;
                            Ok(Type::Any)
                        }
                    }
                    Type::Var(_) => {
                        let elem_ty = self.new_var();
                        if self.find(&index_ty) == Type::Number {
                            self.unify(
                                &left_ty,
                                &Type::Array(Rc::new(RefCell::new(elem_ty.clone()))),
                            )?;
                            Ok(elem_ty)
                        } else {
                            Ok(Type::Any)
                        }
                    }
                    Type::Any => Ok(Type::Any),
                    other => Err(format!("Index operator not supported on type '{}'", other)),
                }
            }
            Expression::IndexAssign { left, index, value } => {
                let left_ty = self.check(left, env)?;
                let index_ty = self.check(index, env)?;
                let val_ty = self.check(value, env)?;

                let resolved_left = self.find(&left_ty);
                match resolved_left {
                    Type::Array(elem_ty) => {
                        self.unify(&index_ty, &Type::Number)?;
                        self.unify(&*elem_ty.borrow(), &val_ty)?;
                        Ok(val_ty)
                    }
                    Type::Hash(fields) => {
                        let static_key = match &**index {
                            Expression::StringLiteral(s) => Some(s.clone()),
                            Expression::Atom(s) => Some(s.clone()),
                            _ => None,
                        };
                        if let Some(key) = static_key {
                            fields.borrow_mut().insert(key, val_ty.clone());
                            Ok(val_ty)
                        } else {
                            self.unify(&index_ty, &Type::String)?;
                            Ok(val_ty)
                        }
                    }
                    Type::Var(_) => {
                        if self.find(&index_ty) == Type::Number {
                            self.unify(
                                &left_ty,
                                &Type::Array(Rc::new(RefCell::new(val_ty.clone()))),
                            )?;
                        } else {
                            let mut fields = HashMap::new();
                            if let Expression::StringLiteral(key) | Expression::Atom(key) = &**index
                            {
                                fields.insert(key.clone(), val_ty.clone());
                            }
                            self.unify(&left_ty, &Type::Hash(Rc::new(RefCell::new(fields))))?;
                        }
                        Ok(val_ty)
                    }
                    Type::Any => Ok(val_ty),
                    other => Err(format!(
                        "Property assignment not supported on type '{}'",
                        other
                    )),
                }
            }
        }
    }
}
