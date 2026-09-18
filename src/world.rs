#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    Forest,
    River,
    Railway,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForestSectionKind {
    Clearing,
    Grove,
    Thicket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForestObstacleKind {
    Tree,
    Rock,
    FallenLog,
    Bush,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForestObstacle {
    pub column: usize,
    pub kind: ForestObstacleKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionKind {
    Forest(ForestSectionKind),
    River,
    Railway,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneKind {
    Grass,
    Stone,
    Water,
    Rail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailwayPhase {
    Rest,
    Warning,
    Crossing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainDirection {
    LeftToRight,
    RightToLeft,
}

impl TrainDirection {
    pub fn sign(self) -> f32 {
        match self {
            Self::LeftToRight => 1.0,
            Self::RightToLeft => -1.0,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::LeftToRight => Self::RightToLeft,
            Self::RightToLeft => Self::LeftToRight,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RailwayState {
    pub phase: RailwayPhase,
    pub elapsed: f32,
    pub direction: TrainDirection,
}

#[derive(Clone, Debug)]
pub struct Lane {
    pub environment: Environment,
    pub section_kind: SectionKind,
    pub section_id: usize,
    pub kind: LaneKind,
    pub obstacles: Vec<ForestObstacle>,
    pub platform_columns: Vec<usize>,
    pub railway: Option<RailwayState>,
}

impl Lane {
    pub fn layout_mask(&self) -> u8 {
        match self.environment {
            Environment::Forest => self
                .obstacles
                .iter()
                .fold(0, |mask, obstacle| mask | (1 << obstacle.column)),
            Environment::River => self
                .platform_columns
                .iter()
                .fold(0, |mask, column| mask | (1 << column)),
            Environment::Railway => 0,
        }
    }

    pub fn blocks_movement(&self, column: usize) -> bool {
        self.environment == Environment::Forest
            && self
                .obstacles
                .iter()
                .any(|obstacle| obstacle.column == column)
    }

    pub fn supports_player(&self, column: usize) -> bool {
        match self.environment {
            Environment::Forest => !self
                .obstacles
                .iter()
                .any(|obstacle| obstacle.column == column),
            Environment::River => self.platform_columns.contains(&column),
            Environment::Railway => true,
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
