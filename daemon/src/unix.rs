




pub struct UnixActor {

}

impl actix::Actor for UnixActor {
    type Context = actix::Context<Self>;

    fn start(self) -> actix::Addr<Self>
    where
        Self: actix::Actor<Context = actix::Context<Self>>,
    {
        actix::Context::new().run(self)
    }
}

