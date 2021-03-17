use cosmwasm_std::Attribute;

pub trait Event {
    fn write_attributes(&self, attributes: &mut Vec<Attribute>);
}
