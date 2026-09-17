#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    Forest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForestSectionKind {
    Clearing,
    Grove,
    Thicket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneKind {
    Grass,
    Stone,
}

#[derive(Clone, Debug)]
pub struct Lane {
    pub environment: Environment,
    pub section_kind: ForestSectionKind,
    pub section_id: usize,
    pub kind: LaneKind,
    pub obstacle_columns: Vec<usize>,
}

impl Lane {
    pub fn obstacle_mask(&self) -> u8 {
        self.obstacle_columns
            .iter()
            .fold(0, |mask, column| mask | (1 << column))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionSignature {
    pub kind: ForestSectionKind,
    pub lane_masks: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub lanes: Vec<Lane>,
    pub signature: SectionSignature,
}

impl Section {
    pub fn new(lanes: Vec<Lane>, kind: ForestSectionKind) -> Self {
        let lane_masks = lanes.iter().map(Lane::obstacle_mask).collect();
        Self {
            lanes,
            signature: SectionSignature { kind, lane_masks },
        }
    }
}
