// Connectors and search integration interfaces
pub trait Connector {
    fn name(&self) -> &'static str;
}
