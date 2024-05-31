use alloc::vec::Vec;

use p3_air::VirtualPairCol;
use p3_field::Field;

#[derive(Clone, Debug)]
pub enum InteractionType {
    Send,
    Receive,
}

#[derive(Clone, Debug)]
pub struct Interaction<F: Field> {
    pub fields: Vec<VirtualPairCol<F>>,
    pub count: VirtualPairCol<F>,
    pub argument_index: usize,
}
