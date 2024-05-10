use alloc::vec::Vec;

use p3_air::VirtualPairCol;
use p3_field::AbstractField;

#[derive(Clone, Debug)]
pub enum InteractionType {
    Send,
    Receive,
}

#[derive(Clone, Debug)]
pub struct Interaction<F: AbstractField> {
    pub fields: Vec<VirtualPairCol<F>>,
    pub count: VirtualPairCol<F>,
    pub argument_index: usize,
}
