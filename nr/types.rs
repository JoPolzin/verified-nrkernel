#[allow(unused_imports)]
use builtin::*;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Some Types
////////////////////////////////////////////////////////////////////////////////////////////////////

/// type of the node id
pub type NodeId = nat;

/// the log index
pub type LogIdx = nat;

/// the request id
pub type ReqId = nat;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Nr State and its operations
////////////////////////////////////////////////////////////////////////////////////////////////////

// the following types are currently arbitrary, the depend on the the actual data structure that
// each replica wraps. The NR crate has this basically as a trait that the data structure must
// implement.

/// represents a replica state
pub struct NRState {
    u: u8,
}

impl NRState {
    #[spec]
    #[verifier(opaque)]
    pub fn init() -> Self {
        NRState { u: 0 }
    }

    /// reads the current state of the replica
    #[spec]
    #[verifier(opaque)]
    pub fn read(&self, op: ReadonlyOp) -> ReturnType {
        ReturnType { u: 0 }
    }

    #[spec]
    #[verifier(opaque)]
    pub fn update(self, op: UpdateOp) -> (Self, ReturnType) {
        (self, ReturnType { u: 0 })
    }
}

// #[spec]
// #[verifier(opaque)]
// pub fn read(state: NRState, op: ReadonlyOp) -> ReturnType {
//     ReturnType { u: 0 }
// }

// #[spec]
// #[verifier(opaque)]
// pub fn update_state(state: NRState, op: UpdateOp) -> (NRState, ReturnType) {
//     (state, ReturnType { u: 0 })
// }


/// represents a update operation on the replica, in NR this is handled by `dispatch_mut`
pub struct UpdateOp {
    u: u8,
}

/// Represents a read-only operation on the replica, in NR this is handled by `dispatch`
pub struct ReadonlyOp {
    u: u8,
}

/// Represents the return type of the read-only operation
#[derive(PartialEq, Eq)]
pub struct ReturnType {
    u: u8,
}

impl Structural for ReturnType {}
