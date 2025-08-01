trait SessionRegistry<S> {
    fn lookup(&self, id: &str) -> Option<&S>;
    fn insert(&mut self, id: String, session: S);
    fn remove(&mut self, id: &str) -> Option<S>;
}
