use rapier2d::na::vector;
use rapier2d::prelude::*;
use std::time::SystemTime;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub struct Client {
    pub id: usize,
    pub last_ping_time: u64,
}
impl Client {
    pub fn new(id: usize) -> Self {
        return Client {
            id,
            last_ping_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
    }
    pub fn update_ping(&mut self) {
        self.last_ping_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}
pub type Games = Arc<RwLock<HashMap<usize, Arc<RwLock<Game>>>>>;

pub trait GameLogic: Send + Sync {
    fn game_type(&self) -> u8;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn update(&mut self, elapsed: f64);
    fn to_bytes(&self) -> Vec<u8>;
}

pub struct Game {
    pub game_type: u8,
    pub last_update_ms: u128,
    pub logic: Box<dyn GameLogic>,
    pub players: Vec<String>,
}

impl Game {
    pub fn new<G: GameLogic + 'static>(logic: G, players: Vec<String>) -> Self {
        let game_type = logic.game_type();

        Self {
            game_type,
            last_update_ms: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            logic: Box::new(logic),
            players,
        }
    }

    pub fn downcast<G: 'static>(&self) -> Option<&G> {
        self.logic.as_any().downcast_ref::<G>()
    }

    pub fn downcast_mut<G: 'static>(&mut self) -> Option<&mut G> {
        self.logic.as_any_mut().downcast_mut::<G>()
    }
    pub fn update(&mut self) {
        let elapsed = self.get_and_update_duration() as f64;
        self.logic.update(elapsed);
    }
    pub fn get_and_update_duration(&mut self) -> u128 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let duration = now - self.last_update_ms;
        self.last_update_ms = now;
        return duration;
    }
}

pub struct SoccerGame {
    pub pipeline: PhysicsPipeline,
    pub integration_parameters: IntegrationParameters,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub colliders: ColliderSet,
    pub narrow_phase: NarrowPhase,
    pub bodies: RigidBodySet,
    pub pucks: [RigidBodyHandle; 10],
    pub ball: RigidBodyHandle,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
}

const RADIUS: f32 = 20.0;
impl SoccerGame {
    pub fn new() -> Self {
        let integration_parameters = IntegrationParameters::default();
        let mut physics_pipeline = PhysicsPipeline::new();
        let mut broad_phase = DefaultBroadPhase::new();
        let mut island_manager = IslandManager::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();
        let mut impulse_joints = ImpulseJointSet::new();
        let mut multibody_joints = MultibodyJointSet::new();
        let mut ccd_solver = CCDSolver::new();
        // Function to create a moving ball
        let mut create_circle = |x: f32, y: f32| -> RigidBodyHandle {
            let body = bodies.insert(
                RigidBodyBuilder::dynamic()
                    .translation(vector![x, y]) // Start position
                    .linvel(vector![0.0, 0.0]) // Initial velocity
                    .linear_damping(0.1) // friction
                    .build(),
            );
            let collider = colliders.insert_with_parent(
                ColliderBuilder::ball(RADIUS) // Circle with radius 1.0
                    .restitution(1.0) // Perfectly elastic bounce
                    .build(),
                body,
                &mut bodies,
            );
            return body;
        };
        let game_width: f32 = 600.0; // X-axis boundaries
        let game_height: f32 = 600.0;
        let mut start: f32 = -200.0;
        let mut pucks = vec![];
        for i in 0..3 {
            let puck = create_circle(-200.0, start);
            pucks.push(puck);
            start += 200.0;
        }
        let puck11 = create_circle(-50.0, -150.0);
        let puck12 = create_circle(-50.0, 150.0);
        pucks.push(puck11);
        pucks.push(puck12);
        start = -200.0;
        for i in 0..3 {
            let puck2 = create_circle(200.0, start);
            pucks.push(puck2);
            start += 200.0;
        }
        let puck21 = create_circle(50.0, 150.0);
        let puck22 = create_circle(50.0, -150.0);
        pucks.push(puck21);
        pucks.push(puck22);

        let ball = create_circle(0.0, 0.0);
        let wall_thickness = 1.0; //

        // Create walls
        let mut create_wall = |position: Vector<f32>, size: Vector<f32>| {
            let body = bodies.insert(RigidBodyBuilder::fixed().translation(position).build());
            colliders.insert_with_parent(
                ColliderBuilder::cuboid(size.x, size.y)
                    .restitution(0.7) // Optional: Bounciness
                    .friction(0.4) // Optional: Surface friction
                    .build(),
                body,
                &mut bodies,
            );
        };

        create_wall(
            vector![-game_width / 2.0 - wall_thickness, 0.0],
            vector![wall_thickness, game_height / 2.0],
        );

        create_wall(
            vector![game_width / 2.0 + wall_thickness, 0.0],
            vector![wall_thickness, game_height / 2.0],
        );

        create_wall(
            vector![0.0, game_height / 2.0 + wall_thickness],
            vector![game_width / 2.0, wall_thickness],
        );

        create_wall(
            vector![0.0, -game_height / 2.0 - wall_thickness],
            vector![game_width / 2.0, wall_thickness],
        );

        SoccerGame {
            pipeline: physics_pipeline,
            colliders,
            bodies,
            pucks: pucks.try_into().unwrap(),
            ball,
            narrow_phase,
            integration_parameters,
            broad_phase,
            island_manager,
            impulse_joints,
            multibody_joints,
            ccd_solver,
        }
    }
}

impl GameLogic for SoccerGame {
    fn game_type(&self) -> u8 {
        return 1;
    }
    fn as_any(&self) -> &dyn std::any::Any {
        return self;
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        return self;
    }
    fn update(&mut self, elapsed: f64) {
        let physics_hooks = ();
        let event_handler = ();
        self.pipeline.step(
            &vector![0.0, 0.0],
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::<u8>::with_capacity(24);
        let mut encode_f32 = |value: f32| data.extend_from_slice(&value.to_le_bytes());
        for puck in &self.pucks {
            if let Some(body) = self.bodies.get(*puck) {
                let pos = body.translation();
                encode_f32(pos.x);
                encode_f32(pos.y);
            }
        }
        if let Some(body) = self.bodies.get(self.ball) {
            let pos = body.translation();
            encode_f32(pos.x);
            encode_f32(pos.y);
        }
        return data;
    }
}
