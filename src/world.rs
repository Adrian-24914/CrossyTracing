#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    Forest,
    River,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForestSectionKind {
    Clearing,
    Grove,
    Thicket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionKind {
    Forest(ForestSectionKind),
    River,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneKind {
    Grass,
    Stone,
    Water,
}

#[derive(Clone, Debug)]
pub struct Lane {
    pub environment: Environment,
    pub section_kind: SectionKind,
    pub section_id: usize,
    pub kind: LaneKind,
    pub obstacle_columns: Vec<usize>,
    pub platform_columns: Vec<usize>,
}

impl Lane {
    pub fn layout_mask(&self) -> u8 {
        let columns = match self.environment {
            Environment::Forest => &self.obstacle_columns,
            Environment::River => &self.platform_columns,
        };
        columns.iter().fold(0, |mask, column| mask | (1 << column))
    }

    pub fn blocks_movement(&self, column: usize) -> bool {
        self.environment == Environment::Forest && self.obstacle_columns.contains(&column)
    }

    pub fn supports_player(&self, column: usize) -> bool {
        match self.environment {
            Environment::Forest => !self.obstacle_columns.contains(&column),
            Environment::River => self.platform_columns.contains(&column),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionSignature {
    pub kind: SectionKind,
    pub lane_masks: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub lanes: Vec<Lane>,
    pub signature: SectionSignature,
}

impl Section {
    pub fn new(lanes: Vec<Lane>, kind: SectionKind) -> Self {
        let lane_masks = lanes.iter().map(Lane::layout_mask).collect();
        Self {
            lanes,
            signature: SectionSignature { kind, lane_masks },
        }
    }
}
