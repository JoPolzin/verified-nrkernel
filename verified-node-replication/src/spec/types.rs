// Verified Node Replication Library
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
#[allow(unused_imports)]
use builtin::*;
use builtin_macros::*;

use crate::Dispatch;

verus! {

////////////////////////////////////////////////////////////////////////////////////////////////////
// Some Types
////////////////////////////////////////////////////////////////////////////////////////////////////
pub use crate::{NodeId, LogIdx, ReqId, ThreadId};

/// This represents an entry in the abstract log
pub tracked struct LogEntry<DT: Dispatch> {
    pub op: DT::WriteOperation,
    pub node_id: NodeId,
}

/// Represents an entry in the log
///
/// datatype ConcreteLogEntry = ConcreteLogEntry(op: nrifc.UpdateOp, node_id: uint64)
pub struct ConcreteLogEntry<DT: Dispatch> {
    pub op: DT::WriteOperation,
    pub node_id: u64,
}

} // verus!
