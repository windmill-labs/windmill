use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowDag {
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
    pub params: Vec<Param>,
    pub source_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Param {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DagNode {
    pub id: String,
    pub node_type: DagNodeType,
    pub label: String,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum DagNodeType {
    Step { name: String, script: String },
    Branch { condition_source: String },
    ParallelStart,
    ParallelEnd,
    LoopStart { iter_source: String },
    LoopEnd,
    Return,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DagEdge {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
