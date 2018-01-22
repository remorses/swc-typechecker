use super::*;
use swc_common::Spanned;

#[parser]
impl<'a, I: Input> Parser<'a, I> {
    /// Original context is restored when returned guard is dropped.
    pub(super) fn with_ctx<'w>(&'w mut self, ctx: Context) -> WithCtx<'w, 'a, I> {
        let orig_ctx = self.ctx;
        self.ctx = ctx;
        WithCtx {
            orig_ctx,
            inner: self,
        }
    }

    /// Original context is restored when returned guard is dropped.
    pub(super) fn include_in_expr<'w>(&'w mut self, include_in_expr: bool) -> WithCtx<'w, 'a, I> {
        let ctx = Context {
            include_in_expr,
            ..self.ctx
        };
        self.with_ctx(ctx)
    }

    /// Parse with given closure
    pub(super) fn parse_with<F, Ret>(&mut self, f: F) -> Ret
    where
        F: FnOnce(&mut Self) -> Ret,
    {
        f(self)
    }

    pub(super) fn spanned<F, Node, Ret>(&mut self, f: F) -> PResult<'a, Node>
    where
        F: FnOnce(&mut Self) -> PResult<'a, Ret>,
        Node: Spanned<Ret>,
    {
        let start = self.input.cur_pos();
        let val = f(self)?;

        let span = span!(start);
        Ok(Spanned::from_unspanned(val, span))
    }
}
pub trait ParseObject<'a, Obj> {
    type Prop;
    fn make_object(span: Span, props: Vec<Self::Prop>) -> Obj;
    fn parse_object_prop(&mut self) -> PResult<'a, Self::Prop>;
}

pub struct WithCtx<'w, 'a: 'w, I: 'w + Input> {
    inner: &'w mut Parser<'a, I>,
    orig_ctx: Context,
}
impl<'w, 'a, I: Input> Deref for WithCtx<'w, 'a, I> {
    type Target = Parser<'a, I>;
    fn deref(&self) -> &Parser<'a, I> {
        &self.inner
    }
}
impl<'w, 'a, I: Input> DerefMut for WithCtx<'w, 'a, I> {
    fn deref_mut(&mut self) -> &mut Parser<'a, I> {
        &mut self.inner
    }
}

impl<'w, 'a, I: Input> Drop for WithCtx<'w, 'a, I> {
    fn drop(&mut self) {
        self.inner.ctx = self.orig_ctx;
    }
}
