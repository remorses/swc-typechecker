#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(specialization)]

use swc_common::Fold;
use swc_ecma_ast::*;
use swc_ts_checker::ModuleTypeInfo;

#[derive(Debug)]
pub struct TypeResolver {
    pub types: ModuleTypeInfo,
}

impl Fold<Module> for TypeResolver {
    fn fold(&mut self, node: Module) -> Module {
        unimplemented!()
    }
}