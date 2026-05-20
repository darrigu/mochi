use crate::ast::{Expression, Pattern, TypeAnn};
use crate::error_reporter::Diagnostic;
use std::collections::{HashMap, HashSet};

macro_rules! wrap_err {
    ($self:expr, $res:expr) => {{
        let line = $self.current_line;
        let col = $self.current_col;
        $res.map_err(|e| crate::error_reporter::Diagnostic {
            line,
            col,
            message: e,
            hint: None,
        })
    }};
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeIdx(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EnvIdx(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Var(usize),
    Number,
    String,
    Atom,
    Array(TypeIdx),
    Hash(HashMap<String, TypeIdx>),
    Function { params: Vec<TypeIdx>, ret: TypeIdx },
    Tuple(Vec<TypeIdx>),
    Any,
}

#[derive(Clone, Debug)]
pub struct TypeEnv {
    pub store: HashMap<String, (TypeIdx, bool)>,
    pub outer: Option<EnvIdx>,
}

pub struct TypeChecker {
    next_var_id: usize,
    substitutions: HashMap<usize, TypeIdx>,
    current_return_type: Option<TypeIdx>,
    pub current_line: usize,
    pub current_col: usize,

    pub types: Vec<Type>,
    pub envs: Vec<TypeEnv>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            next_var_id: 0,
            substitutions: HashMap::new(),
            current_return_type: None,
            current_line: 1,
            current_col: 1,
            types: vec![],
            envs: vec![],
        };
        checker.alloc_type(Type::Number);
        checker
    }

    pub fn alloc_type(&mut self, ty: Type) -> TypeIdx {
        let idx = self.types.len();
        self.types.push(ty);
        TypeIdx(idx)
    }

    pub fn get_type(&self, idx: TypeIdx) -> &Type {
        &self.types[idx.0]
    }

    pub fn get_type_mut(&mut self, idx: TypeIdx) -> &mut Type {
        &mut self.types[idx.0]
    }

    pub fn new_env(&mut self) -> EnvIdx {
        let mut store = HashMap::new();
        let any_ty = self.alloc_type(Type::Any);
        store.insert("print".to_string(), (any_ty, true));

        let idx = self.envs.len();
        self.envs.push(TypeEnv { store, outer: None });
        EnvIdx(idx)
    }

    pub fn new_enclosed_env(&mut self, outer: EnvIdx) -> EnvIdx {
        let idx = self.envs.len();
        self.envs.push(TypeEnv {
            store: HashMap::new(),
            outer: Some(outer),
        });
        EnvIdx(idx)
    }

    pub fn define_var(&mut self, env: EnvIdx, name: String, ty: TypeIdx, is_const: bool) {
        self.envs[env.0].store.insert(name, (ty, is_const));
    }

    pub fn resolve_var(&self, env: EnvIdx, name: &str) -> Option<(TypeIdx, bool)> {
        let mut current = Some(env);
        while let Some(curr_idx) = current {
            let env_node = &self.envs[curr_idx.0];
            if let Some(entry) = env_node.store.get(name) {
                return Some(entry.clone());
            }
            current = env_node.outer;
        }
        None
    }

    fn err<T>(&self, msg: String) -> Result<T, Diagnostic> {
        Err(Diagnostic {
            line: self.current_line,
            col: self.current_col,
            message: msg,
            hint: None,
        })
    }

    fn new_var(&mut self) -> Type {
        let id = self.next_var_id;
        self.next_var_id += 1;
        Type::Var(id)
    }

    pub fn find(&self, idx: TypeIdx) -> TypeIdx {
        match self.get_type(idx) {
            Type::Var(id) => {
                if let Some(&substituted) = self.substitutions.get(id) {
                    self.find(substituted)
                } else {
                    idx
                }
            }
            _ => idx,
        }
    }

    pub fn find_deep(&mut self, idx: TypeIdx) -> TypeIdx {
        let mut visited = HashSet::new();
        self.find_deep_helper(idx, &mut visited)
    }

    fn find_deep_helper(&mut self, idx: TypeIdx, visited: &mut HashSet<TypeIdx>) -> TypeIdx {
        let root = self.find(idx);
        if !visited.insert(root) {
            return root;
        }
        let res = match self.get_type(root).clone() {
            Type::Array(inner) => {
                let resolved_inner = self.find_deep_helper(inner, visited);
                if let Type::Array(i) = self.get_type_mut(root) {
                    *i = resolved_inner;
                }
                root
            }
            Type::Hash(fields) => {
                let mut resolved_fields = HashMap::new();
                for (k, v) in fields {
                    resolved_fields.insert(k, self.find_deep_helper(v, visited));
                }
                if let Type::Hash(f) = self.get_type_mut(root) {
                    *f = resolved_fields;
                }
                root
            }
            Type::Tuple(elements) => {
                let resolved_elements = elements
                    .iter()
                    .map(|&e| self.find_deep_helper(e, visited))
                    .collect();
                if let Type::Tuple(el) = self.get_type_mut(root) {
                    *el = resolved_elements;
                }
                root
            }
            Type::Function { params, ret } => {
                let resolved_params = params
                    .iter()
                    .map(|&p| self.find_deep_helper(p, visited))
                    .collect();
                let resolved_ret = self.find_deep_helper(ret, visited);
                if let Type::Function { params: p, ret: r } = self.get_type_mut(root) {
                    *p = resolved_params;
                    *r = resolved_ret;
                }
                root
            }
            _ => root,
        };
        visited.remove(&root);
        res
    }

    pub fn unify(&mut self, t1: TypeIdx, t2: TypeIdx) -> Result<(), String> {
        let t1 = self.find(t1);
        let t2 = self.find(t2);

        if t1 == t2 {
            return Ok(());
        }

        let ty1 = self.get_type(t1).clone();
        let ty2 = self.get_type(t2).clone();

        match (ty1, ty2) {
            (Type::Number, Type::Number) => Ok(()),
            (Type::String, Type::String) => Ok(()),
            (Type::Atom, Type::Atom) => Ok(()),
            (Type::Var(id1), _) => {
                if self.occurs_in(id1, t2) {
                    return Err("Infinite type detected (occurs check failed)".to_string());
                }
                self.substitutions.insert(id1, t2);
                Ok(())
            }
            (_, Type::Var(id2)) => {
                if self.occurs_in(id2, t1) {
                    return Err("Infinite type detected (occurs check failed)".to_string());
                }
                self.substitutions.insert(id2, t1);
                Ok(())
            }
            (Type::Array(inner1), Type::Array(inner2)) => self.unify(inner1, inner2),
            (Type::Hash(fields1), Type::Hash(fields2)) => {
                let keys1: HashSet<String> = fields1.keys().cloned().collect();
                let keys2: HashSet<String> = fields2.keys().cloned().collect();

                let is_subset_1_in_2 = keys1.is_subset(&keys2);
                let is_subset_2_in_1 = keys2.is_subset(&keys1);

                if !is_subset_1_in_2 && !is_subset_2_in_1 {
                    return Err(format!(
                        "Record type mismatch: incompatible fields. Got {:?}, expected {:?}",
                        keys1, keys2
                    ));
                }

                for k in keys1.intersection(&keys2) {
                    let v1 = *fields1.get(k).unwrap();
                    let v2 = *fields2.get(k).unwrap();
                    self.unify(v1, v2)?;
                }

                if is_subset_1_in_2 {
                    let mut merged = fields1.clone();
                    for k in keys2.difference(&keys1) {
                        let v2 = *fields2.get(k).unwrap();
                        merged.insert(k.clone(), v2);
                    }
                    if let Type::Hash(f1) = self.get_type_mut(t1) {
                        *f1 = merged;
                    }
                } else if is_subset_2_in_1 {
                    let mut merged = fields2.clone();
                    for k in keys1.difference(&keys2) {
                        let v1 = *fields1.get(k).unwrap();
                        merged.insert(k.clone(), v1);
                    }
                    if let Type::Hash(f2) = self.get_type_mut(t2) {
                        *f2 = merged;
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
                for (a1, a2) in p1.into_iter().zip(p2.into_iter()) {
                    self.unify(a1, a2)?;
                }
                self.unify(r1, r2)
            }
            (Type::Tuple(elements1), Type::Tuple(elements2)) => {
                if elements1.len() != elements2.len() {
                    return Err(format!(
                        "Tuple size mismatch: expected size {}, got {}",
                        elements1.len(),
                        elements2.len()
                    ));
                }
                for i in 0..elements1.len() {
                    let e1 = elements1[i];
                    let e2 = elements2[i];
                    if self.unify(e1, e2).is_err() {
                        let any_ty = self.alloc_type(Type::Any);
                        if let Type::Tuple(el1) = self.get_type_mut(t1) {
                            el1[i] = any_ty;
                        }
                        if let Type::Tuple(el2) = self.get_type_mut(t2) {
                            el2[i] = any_ty;
                        }
                    }
                }
                Ok(())
            }
            (Type::Any, _) | (_, Type::Any) => Ok(()),
            _ => Err(format!(
                "Type mismatch: cannot unify '{}' with '{}'",
                self.type_to_string(t1),
                self.type_to_string(t2)
            )),
        }
    }

    fn occurs_in(&self, id: usize, ty: TypeIdx) -> bool {
        let mut visited = HashSet::new();
        self.occurs_in_helper(id, ty, &mut visited)
    }

    fn occurs_in_helper(&self, id: usize, ty: TypeIdx, visited: &mut HashSet<TypeIdx>) -> bool {
        let ty = self.find(ty);
        if !visited.insert(ty) {
            return false;
        }
        let res = match self.get_type(ty) {
            &Type::Var(v) => v == id,
            Type::Array(inner) => self.occurs_in_helper(id, *inner, visited),
            Type::Hash(fields) => fields
                .values()
                .any(|&v| self.occurs_in_helper(id, v, visited)),
            Type::Function { params, ret } => {
                params
                    .iter()
                    .any(|&p| self.occurs_in_helper(id, p, visited))
                    || self.occurs_in_helper(id, *ret, visited)
            }
            Type::Tuple(elements) => elements
                .iter()
                .any(|&e| self.occurs_in_helper(id, e, visited)),
            _ => false,
        };
        visited.remove(&ty);
        res
    }

    pub fn type_to_string(&self, idx: TypeIdx) -> String {
        let mut visited = HashSet::new();
        self.type_to_string_helper(idx, &mut visited)
    }

    fn type_to_string_helper(&self, idx: TypeIdx, visited: &mut HashSet<TypeIdx>) -> String {
        let idx = self.find(idx);
        if !visited.insert(idx) {
            return "...".to_string();
        }
        let res = match self.get_type(idx) {
            &Type::Var(id) => format!("'t{}", id),
            Type::Number => "Number".to_string(),
            Type::String => "String".to_string(),
            Type::Atom => "Atom".to_string(),
            Type::Any => "Any".to_string(),
            Type::Array(inner) => format!("Array<{}>", self.type_to_string_helper(*inner, visited)),
            Type::Hash(fields) => {
                let pairs: Vec<String> = fields
                    .iter()
                    .map(|(k, &v)| format!("{}: {}", k, self.type_to_string_helper(v, visited)))
                    .collect();
                format!("{{ {} }}", pairs.join(", "))
            }
            Type::Function { params, ret } => {
                let param_strs: Vec<String> = params
                    .iter()
                    .map(|&p| self.type_to_string_helper(p, visited))
                    .collect();
                format!(
                    "({}) -> {}",
                    param_strs.join(", "),
                    self.type_to_string_helper(*ret, visited)
                )
            }
            Type::Tuple(elements) => {
                let items: Vec<String> = elements
                    .iter()
                    .map(|&item| self.type_to_string_helper(item, visited))
                    .collect();
                if elements.len() == 1 {
                    format!("({},)", items[0])
                } else {
                    format!("({})", items.join(", "))
                }
            }
        };
        visited.remove(&idx);
        res
    }

    fn map_type_ann(&mut self, ann: &TypeAnn) -> TypeIdx {
        match ann {
            TypeAnn::Number => self.alloc_type(Type::Number),
            TypeAnn::String => self.alloc_type(Type::String),
            TypeAnn::Atom => self.alloc_type(Type::Atom),
            TypeAnn::Any => self.alloc_type(Type::Any),
            TypeAnn::Array(inner) => {
                let mapped_inner = self.map_type_ann(inner);
                self.alloc_type(Type::Array(mapped_inner))
            }
            TypeAnn::Hash(fields) => {
                let mut mapped_fields = HashMap::new();
                for (k, v) in fields {
                    mapped_fields.insert(k.clone(), self.map_type_ann(v));
                }
                self.alloc_type(Type::Hash(mapped_fields))
            }
            TypeAnn::Function { params, ret } => {
                let mapped_params = params.iter().map(|p| self.map_type_ann(p)).collect();
                let mapped_ret = self.map_type_ann(ret);
                self.alloc_type(Type::Function {
                    params: mapped_params,
                    ret: mapped_ret,
                })
            }
            TypeAnn::Tuple(elements) => {
                let mapped_elements = elements.iter().map(|e| self.map_type_ann(e)).collect();
                self.alloc_type(Type::Tuple(mapped_elements))
            }
        }
    }

    fn check_pattern(
        &mut self,
        pattern: &Pattern,
        subject_ty: TypeIdx,
        env: EnvIdx,
    ) -> Result<(), Diagnostic> {
        let subject_ty = self.find(subject_ty);
        let s_ty = self.get_type(subject_ty).clone();
        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Identifier(name) => {
                self.define_var(env, name.clone(), subject_ty, false);
                Ok(())
            }
            Pattern::Number(_) => {
                let num_ty = self.alloc_type(Type::Number);
                wrap_err!(self, self.unify(subject_ty, num_ty))
            }
            Pattern::StringLiteral(_) => {
                let str_ty = self.alloc_type(Type::String);
                wrap_err!(self, self.unify(subject_ty, str_ty))
            }
            Pattern::Atom(_) => {
                let atom_ty = self.alloc_type(Type::Atom);
                wrap_err!(self, self.unify(subject_ty, atom_ty))
            }
            Pattern::Tuple(elements) => match s_ty {
                Type::Tuple(expected_elements) => {
                    if elements.len() != expected_elements.len() {
                        return self.err(format!(
                            "Tuple pattern size mismatch: expected size {}, got {}",
                            expected_elements.len(),
                            elements.len()
                        ));
                    }
                    for (p, &expected_el_ty) in elements.iter().zip(expected_elements.iter()) {
                        self.check_pattern(p, expected_el_ty, env)?;
                    }
                    Ok(())
                }
                Type::Var(_) => {
                    let mut inner_vars = vec![];
                    for _ in 0..elements.len() {
                        let v = self.new_var();
                        inner_vars.push(self.alloc_type(v));
                    }

                    let tuple_ty = self.alloc_type(Type::Tuple(inner_vars.clone()));
                    wrap_err!(self, self.unify(subject_ty, tuple_ty))?;
                    for (p, &inner_v) in elements.iter().zip(inner_vars.iter()) {
                        self.check_pattern(p, inner_v, env)?;
                    }

                    Ok(())
                }
                Type::Any => {
                    let any_ty = self.alloc_type(Type::Any);
                    for p in elements {
                        self.check_pattern(p, any_ty, env)?;
                    }
                    Ok(())
                }
                _ => {
                    let ty_str = self.type_to_string(subject_ty);
                    self.err(format!(
                        "Cannot match tuple pattern against type '{}'",
                        ty_str
                    ))
                }
            },
        }
    }

    pub fn check_expected(
        &mut self,
        expr: &Expression,
        env: EnvIdx,
        expected: TypeIdx,
    ) -> Result<TypeIdx, Diagnostic> {
        if let Expression::Loc {
            line,
            col,
            expr: inner,
        } = expr
        {
            self.current_line = *line;
            self.current_col = *col;
            return self.check_expected(inner, env, expected);
        }

        let expected = self.find(expected);
        let exp_ty = self.get_type(expected).clone();
        match (expr, &exp_ty) {
            (&Expression::Array(ref elements), &Type::Array(expected_elem_ty)) => {
                for el in elements {
                    let el_ty = self.check(el, env)?;
                    wrap_err!(self, self.unify(el_ty, expected_elem_ty))?;
                }
                Ok(expected)
            }
            (&Expression::Hash(ref pairs), &Type::Hash(ref expected_fields)) => {
                let actual_keys: HashSet<String> = pairs.iter().map(|(k, _)| k.clone()).collect();
                let expected_keys: HashSet<String> = expected_fields.keys().cloned().collect();

                if !expected_keys.is_subset(&actual_keys) {
                    let missing: Vec<String> =
                        expected_keys.difference(&actual_keys).cloned().collect();
                    return self.err(format!(
                        "Record type mismatch: missing required fields {:?}",
                        missing
                    ));
                }

                let mut actual_fields = HashMap::new();
                for (key, val) in pairs {
                    let val_ty = if let Some(&expected_val_ty) = expected_fields.get(key) {
                        self.check_expected(val, env, expected_val_ty)?
                    } else {
                        self.check(val, env)?
                    };
                    actual_fields.insert(key.clone(), val_ty);
                }

                let actual_hash_ty = self.alloc_type(Type::Hash(actual_fields));
                wrap_err!(self, self.unify(actual_hash_ty, expected))?;

                Ok(actual_hash_ty)
            }
            (&Expression::Tuple(ref elements), &Type::Tuple(ref expected_elements)) => {
                if elements.len() != expected_elements.len() {
                    return self.err(format!(
                        "Tuple size mismatch: expected size {}, got {}",
                        expected_elements.len(),
                        elements.len()
                    ));
                }
                let mut actual_elements = vec![];
                for (el, &expected_el_ty) in elements.iter().zip(expected_elements.iter()) {
                    let act_el = self.check_expected(el, env, expected_el_ty)?;
                    actual_elements.push(act_el);
                }
                Ok(self.alloc_type(Type::Tuple(actual_elements)))
            }
            _ => {
                let val_ty = self.check(expr, env)?;
                wrap_err!(self, self.unify(val_ty, expected))?;
                Ok(val_ty)
            }
        }
    }

    pub fn check(&mut self, expr: &Expression, env: EnvIdx) -> Result<TypeIdx, Diagnostic> {
        if let Expression::Loc {
            line,
            col,
            expr: inner,
        } = expr
        {
            self.current_line = *line;
            self.current_col = *col;
            return self.check(inner, env);
        }

        match expr {
            Expression::Identifier(name) => {
                if let Some((ty, _)) = self.resolve_var(env, name) {
                    Ok(ty)
                } else {
                    self.err(format!("Undefined variable: '{}'", name))
                }
            }
            Expression::Number(_) => Ok(self.alloc_type(Type::Number)),
            Expression::StringLiteral(_) => Ok(self.alloc_type(Type::String)),
            Expression::Atom(_) => Ok(self.alloc_type(Type::Atom)),
            Expression::Array(elements) => {
                let elem_var = self.new_var();
                let elem_ty = self.alloc_type(elem_var);
                let mut failed_homogeneous = false;

                for el in elements {
                    let el_ty = self.check(el, env)?;
                    if self.unify(elem_ty, el_ty).is_err() {
                        failed_homogeneous = true;
                    }
                }

                if failed_homogeneous {
                    let any_ty = self.alloc_type(Type::Any);
                    Ok(self.alloc_type(Type::Array(any_ty)))
                } else {
                    let found = self.find(elem_ty);
                    Ok(self.alloc_type(Type::Array(found)))
                }
            }
            Expression::Hash(pairs) => {
                let mut fields = HashMap::new();
                for (key, val) in pairs {
                    let val_ty = self.check(val, env)?;
                    fields.insert(key.clone(), val_ty);
                }
                Ok(self.alloc_type(Type::Hash(fields)))
            }
            Expression::Prefix { operator, right } => {
                let right_ty = self.check(right, env)?;
                match operator.as_str() {
                    "-" => {
                        let num_ty = self.alloc_type(Type::Number);
                        wrap_err!(self, self.unify(right_ty, num_ty))?;
                        Ok(num_ty)
                    }
                    "!" => Ok(self.alloc_type(Type::Atom)),
                    _ => self.err(format!("Unknown prefix operator: '{}'", operator)),
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
                        wrap_err!(self, self.unify(left_ty, right_ty))?;
                        let resolved = self.find(left_ty);
                        match self.get_type(resolved) {
                            Type::Number | Type::String | Type::Var(_) | Type::Any => Ok(left_ty),
                            _ => {
                                let ty_str = self.type_to_string(resolved);
                                self.err(format!(
                                    "Operator '+' is not supported for type '{}'",
                                    ty_str
                                ))
                            }
                        }
                    }
                    "-" | "*" | "/" => {
                        let num_ty = self.alloc_type(Type::Number);
                        wrap_err!(self, self.unify(left_ty, num_ty))?;
                        wrap_err!(self, self.unify(right_ty, num_ty))?;
                        Ok(num_ty)
                    }
                    "==" | "!=" => {
                        wrap_err!(self, self.unify(left_ty, right_ty))?;
                        Ok(self.alloc_type(Type::Atom))
                    }
                    ">" | "<" => {
                        let num_ty = self.alloc_type(Type::Number);
                        wrap_err!(self, self.unify(left_ty, num_ty))?;
                        wrap_err!(self, self.unify(right_ty, num_ty))?;
                        Ok(self.alloc_type(Type::Atom))
                    }
                    _ => self.err(format!("Unknown infix operator: '{}'", operator)),
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
                    wrap_err!(self, self.unify(cons_ty, alt_ty))?;
                    Ok(cons_ty)
                } else {
                    Ok(self.alloc_type(Type::Any))
                }
            }
            Expression::Block(expressions) => {
                let block_env = self.new_enclosed_env(env);
                if expressions.is_empty() {
                    return Ok(self.alloc_type(Type::Atom));
                }
                let mut last_ty = self.alloc_type(Type::Atom);
                for expr in expressions {
                    last_ty = self.check(expr, block_env)?;
                }
                Ok(last_ty)
            }
            Expression::Let {
                name,
                type_ann,
                value,
            } => {
                if let Some(ann) = type_ann {
                    let expected_ty = self.map_type_ann(ann);
                    let val_ty = if let Expression::Function { .. } = unwrap_loc(value) {
                        self.define_var(env, name.clone(), expected_ty, false);
                        self.check_expected(value, env, expected_ty)?
                    } else {
                        self.check_expected(value, env, expected_ty)?
                    };
                    self.define_var(env, name.clone(), expected_ty, false);
                    Ok(val_ty)
                } else {
                    let val_ty = if let Expression::Function { .. } = unwrap_loc(value) {
                        let placeholder_var = self.new_var();
                        let placeholder = self.alloc_type(placeholder_var);
                        self.define_var(env, name.clone(), placeholder, false);
                        let actual_ty = self.check(value, env)?;
                        wrap_err!(self, self.unify(placeholder, actual_ty))?;
                        actual_ty
                    } else {
                        self.check(value, env)?
                    };
                    self.define_var(env, name.clone(), val_ty, false);
                    Ok(val_ty)
                }
            }
            Expression::Const {
                name,
                type_ann,
                value,
            } => {
                if let Some(ann) = type_ann {
                    let expected_ty = self.map_type_ann(ann);
                    let val_ty = if let Expression::Function { .. } = unwrap_loc(value) {
                        self.define_var(env, name.clone(), expected_ty, true);
                        self.check_expected(value, env, expected_ty)?
                    } else {
                        self.check_expected(value, env, expected_ty)?
                    };
                    self.define_var(env, name.clone(), expected_ty, true);
                    Ok(val_ty)
                } else {
                    let val_ty = if let Expression::Function { .. } = unwrap_loc(value) {
                        let placeholder_var = self.new_var();
                        let placeholder = self.alloc_type(placeholder_var);
                        self.define_var(env, name.clone(), placeholder, true);
                        let actual_ty = self.check(value, env)?;
                        wrap_err!(self, self.unify(placeholder, actual_ty))?;
                        actual_ty
                    } else {
                        self.check(value, env)?
                    };
                    self.define_var(env, name.clone(), val_ty, true);
                    Ok(val_ty)
                }
            }
            Expression::Assign { name, value } => {
                let resolved = self.resolve_var(env, name);
                if let Some((existing_ty, is_const)) = resolved {
                    if is_const {
                        return self.err(format!("Cannot reassign constant '{}'", name));
                    }
                    let val_ty = self.check(value, env)?;
                    wrap_err!(self, self.unify(existing_ty, val_ty))?;
                    Ok(val_ty)
                } else {
                    self.err(format!("Undefined variable: '{}'", name))
                }
            }
            Expression::Return(expr) => {
                let expr_ty = self.check(expr, env)?;
                if let Some(expected_ty) = self.current_return_type {
                    wrap_err!(self, self.unify(expr_ty, expected_ty))?;
                } else {
                    return self.err("Return statement outside function context".to_string());
                }
                Ok(expr_ty)
            }
            Expression::Function {
                parameters,
                return_type,
                body,
            } => {
                let fn_env = self.new_enclosed_env(env);
                let mut param_types = vec![];

                for (param, type_ann) in parameters {
                    let p_ty = if let Some(ann) = type_ann {
                        self.map_type_ann(ann)
                    } else {
                        let v = self.new_var();
                        self.alloc_type(v)
                    };
                    self.define_var(fn_env, param.clone(), p_ty, false);
                    param_types.push(p_ty);
                }

                let prev_ret = self.current_return_type;
                let expected_ret = if let Some(ann) = return_type {
                    self.map_type_ann(ann)
                } else {
                    let v = self.new_var();
                    self.alloc_type(v)
                };
                self.current_return_type = Some(expected_ret);

                let mut body_ty = self.alloc_type(Type::Atom);
                for expr in body {
                    body_ty = self.check(expr, fn_env)?;
                }

                wrap_err!(self, self.unify(body_ty, expected_ret))?;
                let final_ret = self.find_deep(expected_ret);
                self.current_return_type = prev_ret;

                let final_params = param_types.iter().map(|&p| self.find_deep(p)).collect();

                Ok(self.alloc_type(Type::Function {
                    params: final_params,
                    ret: final_ret,
                }))
            }
            Expression::Call {
                function,
                arguments,
            } => {
                let fn_ty = self.check(function, env)?;
                let resolved_fn = self.find_deep(fn_ty);

                match self.get_type(resolved_fn).clone() {
                    Type::Function { params, ret } => {
                        if params.len() != arguments.len() {
                            return self.err(format!(
                                "Function arity mismatch: expected {} arguments, got {}",
                                params.len(),
                                arguments.len()
                            ));
                        }
                        let mut arg_types = vec![];
                        for (arg, &param_ty) in arguments.iter().zip(params.iter()) {
                            arg_types.push(self.check_expected(arg, env, param_ty)?);
                        }
                        Ok(ret)
                    }
                    Type::Any => {
                        let mut arg_types = vec![];
                        for arg in arguments {
                            arg_types.push(self.check(arg, env)?);
                        }
                        Ok(self.alloc_type(Type::Any))
                    }
                    _ => {
                        let mut arg_types = vec![];
                        for arg in arguments {
                            arg_types.push(self.check(arg, env)?);
                        }
                        let ret_var = self.new_var();
                        let ret_ty = self.alloc_type(ret_var);
                        let mapped_fn = self.alloc_type(Type::Function {
                            params: arg_types,
                            ret: ret_ty,
                        });
                        wrap_err!(self, self.unify(fn_ty, mapped_fn))?;
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
                let mut checked_args = vec![left_ty];

                for arg in arguments {
                    checked_args.push(self.check(arg, env)?);
                }

                let ret_var = self.new_var();
                let ret_ty = self.alloc_type(ret_var);
                let resolved_left = self.find_deep(left_ty);

                match self.get_type(resolved_left).clone() {
                    Type::Hash(fields) => {
                        if let Some(&method_ty) = fields.get(method) {
                            let resolved_method = self.find_deep(method_ty);
                            match self.get_type(resolved_method).clone() {
                                Type::Function { params, ret } => {
                                    if params.len() != arguments.len() + 1 {
                                        return self.err(format!(
                                            "Function arity mismatch: expected {} arguments, got {}",
                                            params.len() - 1,
                                            arguments.len()
                                        ));
                                    }
                                    wrap_err!(self, self.unify(left_ty, params[0]))?;

                                    for (arg, &param_ty) in
                                        arguments.iter().zip(params.iter().skip(1))
                                    {
                                        self.check_expected(arg, env, param_ty)?;
                                    }
                                    Ok(ret)
                                }
                                _ => {
                                    let mut checked_args = vec![left_ty];
                                    for arg in arguments {
                                        checked_args.push(self.check(arg, env)?);
                                    }
                                    let m_fn = self.alloc_type(Type::Function {
                                        params: checked_args,
                                        ret: ret_ty,
                                    });
                                    wrap_err!(self, self.unify(method_ty, m_fn))?;
                                    Ok(ret_ty)
                                }
                            }
                        } else {
                            let mut checked_args = vec![left_ty];
                            let mut method_args = checked_args.clone();
                            let any_ty = self.alloc_type(Type::Any);
                            method_args[0] = any_ty;
                            for arg in arguments {
                                let arg_ty = self.check(arg, env)?;
                                checked_args.push(arg_ty);
                                method_args.push(arg_ty);
                            }
                            let method_ty = self.alloc_type(Type::Function {
                                params: method_args,
                                ret: ret_ty,
                            });
                            if let Type::Hash(f) = self.get_type_mut(resolved_left) {
                                f.insert(method.clone(), method_ty);
                            }
                            Ok(ret_ty)
                        }
                    }
                    Type::Any => Ok(self.alloc_type(Type::Any)),
                    Type::Var(_) => {
                        let mut fields = HashMap::new();
                        let mut method_args = checked_args.clone();
                        let any_ty = self.alloc_type(Type::Any);
                        method_args[0] = any_ty;

                        let method_ty = self.alloc_type(Type::Function {
                            params: method_args,
                            ret: ret_ty,
                        });
                        fields.insert(method.clone(), method_ty);
                        let hash_ty = self.alloc_type(Type::Hash(fields));
                        wrap_err!(self, self.unify(left_ty, hash_ty))?;
                        Ok(ret_ty)
                    }
                    _other => {
                        let ty_str = self.type_to_string(resolved_left);
                        self.err(format!(
                            "Method call failed: receiver is of type '{}', which is not an object",
                            ty_str
                        ))
                    }
                }
            }
            Expression::Index { left, index } => {
                let left_ty = self.check(left, env)?;
                let index_ty = self.check(index, env)?;

                let resolved_left = self.find_deep(left_ty);
                match self.get_type(resolved_left).clone() {
                    Type::Array(elem_ty) => {
                        let num_ty = self.alloc_type(Type::Number);
                        wrap_err!(self, self.unify(index_ty, num_ty))?;
                        Ok(elem_ty)
                    }
                    Type::Hash(fields) => {
                        let static_key = match unwrap_loc(index) {
                            &Expression::StringLiteral(ref s) => Some(s.clone()),
                            &Expression::Atom(ref s) => Some(s.clone()),
                            _ => None,
                        };
                        if let Some(key) = static_key {
                            if let Some(&ty) = fields.get(&key) {
                                Ok(ty)
                            } else {
                                let new_prop_var = self.new_var();
                                let new_prop_ty = self.alloc_type(new_prop_var);
                                if let Type::Hash(f) = self.get_type_mut(resolved_left) {
                                    f.insert(key, new_prop_ty);
                                }
                                Ok(new_prop_ty)
                            }
                        } else {
                            let str_ty = self.alloc_type(Type::String);
                            wrap_err!(self, self.unify(index_ty, str_ty))?;
                            Ok(self.alloc_type(Type::Any))
                        }
                    }
                    Type::Var(_) => {
                        let elem_var = self.new_var();
                        let elem_ty = self.alloc_type(elem_var);
                        let resolved_idx = self.find(index_ty);
                        if self.get_type(resolved_idx) == &Type::Number {
                            let arr_ty = self.alloc_type(Type::Array(elem_ty));
                            wrap_err!(self, self.unify(left_ty, arr_ty,))?;
                            Ok(elem_ty)
                        } else {
                            Ok(self.alloc_type(Type::Any))
                        }
                    }
                    Type::Tuple(elements) => {
                        let num_ty = self.alloc_type(Type::Number);
                        wrap_err!(self, self.unify(index_ty, num_ty))?;
                        let static_idx = match unwrap_loc(index) {
                            Expression::Number(n) => Some(*n as usize),
                            _ => None,
                        };
                        if let Some(i) = static_idx {
                            if i < elements.len() {
                                Ok(elements[i])
                            } else {
                                self.err(format!("Tuple index {} out of bounds", i))
                            }
                        } else {
                            Ok(self.alloc_type(Type::Any))
                        }
                    }
                    Type::Any => Ok(self.alloc_type(Type::Any)),
                    _other => {
                        let ty_str = self.type_to_string(resolved_left);
                        self.err(format!("Index operator not supported on type '{}'", ty_str))
                    }
                }
            }
            Expression::IndexAssign { left, index, value } => {
                let left_ty = self.check(left, env)?;
                let index_ty = self.check(index, env)?;
                let val_ty = self.check(value, env)?;

                let resolved_left = self.find_deep(left_ty);
                match self.get_type(resolved_left).clone() {
                    Type::Array(elem_ty) => {
                        let num_ty = self.alloc_type(Type::Number);
                        wrap_err!(self, self.unify(index_ty, num_ty))?;
                        wrap_err!(self, self.unify(elem_ty, val_ty))?;
                        Ok(val_ty)
                    }
                    Type::Hash(_) => {
                        let static_key = match unwrap_loc(index) {
                            &Expression::StringLiteral(ref s) => Some(s.clone()),
                            &Expression::Atom(ref s) => Some(s.clone()),
                            _ => None,
                        };
                        if let Some(key) = static_key {
                            if let Type::Hash(f) = self.get_type_mut(resolved_left) {
                                f.insert(key, val_ty);
                            }
                            Ok(val_ty)
                        } else {
                            let str_ty = self.alloc_type(Type::String);
                            wrap_err!(self, self.unify(index_ty, str_ty))?;
                            Ok(val_ty)
                        }
                    }
                    Type::Var(_) => {
                        let resolved_idx = self.find(index_ty);
                        if self.get_type(resolved_idx) == &Type::Number {
                            let arr_ty = self.alloc_type(Type::Array(val_ty));
                            wrap_err!(self, self.unify(left_ty, arr_ty))?;
                        } else {
                            let mut fields = HashMap::new();
                            if let &Expression::StringLiteral(ref key)
                            | &Expression::Atom(ref key) = unwrap_loc(index)
                            {
                                fields.insert(key.clone(), val_ty);
                            }
                            let hash_ty = self.alloc_type(Type::Hash(fields));
                            wrap_err!(self, self.unify(left_ty, hash_ty))?;
                        }
                        Ok(val_ty)
                    }
                    Type::Tuple(_) => self.err("Tuples are immutable".to_string()),
                    Type::Any => Ok(val_ty),
                    _other => {
                        let ty_str = self.type_to_string(resolved_left);
                        self.err(format!(
                            "Property assignment not supported on type '{}'",
                            ty_str
                        ))
                    }
                }
            }
            Expression::Loop { body } => {
                let _body_ty = self.check(body, env)?;
                Ok(self.alloc_type(Type::Atom))
            }
            Expression::While { condition, body } => {
                let _cond_ty = self.check(condition, env)?;
                let _body_ty = self.check(body, env)?;
                Ok(self.alloc_type(Type::Atom))
            }
            Expression::For {
                element,
                iterable,
                body,
            } => {
                let iter_ty = self.check(iterable, env)?;
                let resolved_iter = self.find_deep(iter_ty);

                let loop_env = self.new_enclosed_env(env);

                let elem_ty = match self.get_type(resolved_iter).clone() {
                    Type::Array(inner) => inner,
                    Type::Var(_) => {
                        let inner_var = self.new_var();
                        let inner = self.alloc_type(inner_var);
                        let arr_ty = self.alloc_type(Type::Array(inner));
                        wrap_err!(self, self.unify(iter_ty, arr_ty))?;
                        inner
                    }
                    _ => self.alloc_type(Type::Any),
                };

                self.define_var(loop_env, element.clone(), elem_ty, false);
                let _body_ty = self.check(body, loop_env)?;
                Ok(self.alloc_type(Type::Atom))
            }
            Expression::ForHash {
                key,
                value,
                iterable,
                body,
            } => {
                let iter_ty = self.check(iterable, env)?;
                let resolved_iter = self.find_deep(iter_ty);

                let loop_env = self.new_enclosed_env(env);

                match self.get_type(resolved_iter) {
                    Type::Hash(_fields) => {}
                    Type::Var(_) => {
                        let hash_ty = self.alloc_type(Type::Hash(HashMap::new()));
                        wrap_err!(self, self.unify(iter_ty, hash_ty))?;
                    }
                    _ => {}
                };

                let str_ty = self.alloc_type(Type::String);
                let any_ty = self.alloc_type(Type::Any);
                self.define_var(loop_env, key.clone(), str_ty, false);
                self.define_var(loop_env, value.clone(), any_ty, false);
                let _body_ty = self.check(body, loop_env)?;
                Ok(self.alloc_type(Type::Atom))
            }
            Expression::Tuple(elements) => {
                let mut element_types = vec![];
                for el in elements {
                    element_types.push(self.check(el, env)?);
                }
                Ok(self.alloc_type(Type::Tuple(element_types)))
            }
            Expression::Match { subject, cases } => {
                let subject_ty = self.check(subject, env)?;
                let ret_var = self.new_var();
                let ret_ty = self.alloc_type(ret_var);

                for case in cases {
                    let case_env = self.new_enclosed_env(env);
                    self.check_pattern(&case.pattern, subject_ty, case_env)?;

                    if let Some(guard_expr) = &case.guard {
                        let _guard_ty = self.check(guard_expr, case_env)?;
                    }

                    let body_ty = self.check(&case.body, case_env)?;
                    wrap_err!(self, self.unify(body_ty, ret_ty))?;
                }

                Ok(self.find_deep(ret_ty))
            }
            Expression::Loc { .. } => unreachable!(),
        }
    }
}

fn unwrap_loc(expr: &Expression) -> &Expression {
    match expr {
        Expression::Loc { expr, .. } => unwrap_loc(expr),
        _ => expr,
    }
}
