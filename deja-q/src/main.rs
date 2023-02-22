use clap::{Parser, arg, Subcommand};
use postcard::{to_allocvec, from_bytes};
use rand::{prelude::SliceRandom, rngs::SmallRng, Rng, SeedableRng};
use serde::{Serialize, Deserialize};
use std::{
    collections::HashMap,
    io::{stdout, Write, Read},
    thread::sleep,
    time::Duration, fs::{File, self},
};

const DEFAULT_QUALITY: f64 = 0.0;

// Use non-recursive heaps algorithm to generate all permutations of directions to try
const DIR_PERMUTATIONS: [[usize; 4]; 24] = {
    let mut generated: [[usize; 4]; 24] = [[0; 4]; 24];
    let mut to_permute = [0, 1, 2, 3];
    // "push" initial combo
    generated[0] = to_permute;
    let mut gen_idx = 1;
    let mut c = [0; 4];
    let mut i = 0;
    while i < to_permute.len() {
        if c[i] < i {
            let idx1 = if i % 2 == 0 { 0 } else { c[i] };
            let tmp = to_permute[idx1];
            to_permute[idx1] = to_permute[i];
            to_permute[i] = tmp;
            generated[gen_idx] = to_permute;
            gen_idx += 1;
            c[i] = c[i] + 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    generated
};

#[derive(Copy, Clone, PartialEq)]
enum Background {
    Empty,
    Wall,
}

fn clear_screen() {
    print!("{esc}c", esc = 27 as char);
}

struct Board {
    tiles: Vec<Vec<Background>>,
    ghosts: Vec<(usize, usize)>,
    pacman: (usize, usize),
    rng: SmallRng,
}

trait PacmanAi {
    /// Should only move at most once! Otherwise it is a BUUUUUUG
    fn tick_pacman(&mut self, board: &mut Board);
}

impl Board {
    fn new(width: usize, height: usize) -> Self {
        Board {
            tiles: vec![vec![Background::Empty; width]; height],
            ghosts: vec![],
            pacman: (5, 5),
            rng: SmallRng::from_entropy(),
        }
    }

    fn initialize(width: usize, height: usize, ghost_num: usize) -> Self {
        let mut board = Board::new(width, height);
        for x in 0..width {
            board.tiles[0][x] = Background::Wall;
            board.tiles[height - 1][x] = Background::Wall;
        }
        for y in 1..(height - 1) {
            board.tiles[y][0] = Background::Wall;
            board.tiles[y][width - 1] = Background::Wall;
        }
        while board.ghosts.len() < ghost_num {
            let pos = board.rand_empty_pos();
            board.ghosts.push(pos);
        }
        board.pacman = board.rand_empty_pos();
        board
    }

    fn rand_empty_pos(&mut self) -> (usize, usize) {
        let height = self.tiles.len();
        let width = self.tiles[0].len();
        loop {
            let x = self.rng.gen_range(0..width);
            let y = self.rng.gen_range(0..height);
            if self.tiles[y][x] == Background::Empty && !self.ghosts.contains(&(x, y)) {
                return (x, y);
            }
        }
    }

    fn render(&self) {
        clear_screen();
        for y in 0..self.tiles.len() {
            for x in 0..self.tiles[0].len() {
                print!(
                    "{}",
                    match self.tiles[y][x] {
                        Background::Empty => {
                            if self.pacman == (x, y) {
                                '🤪'
                            } else if self.ghosts.contains(&(x, y)) {
                                '👻'
                            } else {
                                '　'
                            }
                        }
                        Background::Wall => '⬜',
                    }
                );
            }
            println!();
        }
    }

    fn has_ghost_at(&self, x: usize, y: usize) -> bool {
        self.ghosts.contains(&(x, y))
    }

    fn tick_ghosts(&mut self) {
        for i in 0..self.ghosts.len() {
            let (x, y) = self.ghosts[i];
            let moves = [(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)];
            let order = DIR_PERMUTATIONS.choose(&mut self.rng).unwrap();
            for q in order.iter() {
                let (xi, yi) = moves[*q];
                if matches!(self.tiles[yi][xi], Background::Empty) && !self.has_ghost_at(xi, yi) {
                    self.ghosts[i] = (xi, yi);
                    break;
                }
            }
        }
    }

    fn is_game_over(&self) -> bool {
        let (x, y) = self.pacman;
        self.tiles[y][x] == Background::Wall || self.ghosts.contains(&self.pacman)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
enum Action {
    Idle,
    Up,
    Down,
    Left,
    Right,
}

impl Action {
    fn random() -> Action {
        let mut rng = SmallRng::from_entropy();
        *[
            Action::Idle,
            Action::Up,
            Action::Down,
            Action::Left,
            Action::Right,
        ]
        .choose(&mut rng)
        .unwrap()
    }
}

type StateActions = [(Action, f64); 5];

#[derive(Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
enum Tile {
    Empty,
    Ghost,
    Wall,
}

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct QState {
    // 5*5 - 1 view surrounding pacman
    surroundings: [Tile; 24],
}

struct PaQman {
    q_table: HashMap<QState, StateActions>,
    learning_rate: f64,
    /// Changes over time - odds that we'll decide to take a random action
    /// Between [0.0, 1.0]
    discount_rate: f64,
}

impl PaQman {
    fn new() -> Self {
        PaQman {
            q_table: HashMap::new(),
            learning_rate: 0.7, // Apparently this is a good learning rate
            discount_rate: 0.95,
        }
    }

    /// Bellman's algorithm to update table
    fn update_q_table(&mut self, reward: f64, state: &QState, new_state: &QState, action: Action) {
        let prev_quality = self.get_quality(state, action);
        let best_next_q = self.get_best_action(new_state).1;
        let new_quality = prev_quality
            + self.learning_rate * (reward + self.discount_rate * best_next_q - prev_quality);
        self.set_quality(state, action, new_quality);
    }

    fn get_quality(&self, q_state: &QState, action: Action) -> f64 {
        match self.q_table.get(q_state) {
            Some(actions) => actions.iter().find(|(act, _)| *act == action).unwrap().1,
            None => DEFAULT_QUALITY,
        }
    }

    fn set_quality(&mut self, q_state: &QState, action: Action, quality: f64) {
        if !self.q_table.contains_key(q_state) {
            let actions = [
                (Action::Idle, DEFAULT_QUALITY),
                (Action::Up, DEFAULT_QUALITY),
                (Action::Down, DEFAULT_QUALITY),
                (Action::Left, DEFAULT_QUALITY),
                (Action::Right, DEFAULT_QUALITY),
            ];
            self.q_table.insert(q_state.clone(), actions);
        };
        let actions = self.q_table.get_mut(q_state).unwrap();
        actions
            .iter_mut()
            .find(|(act, _)| *act == action)
            .unwrap()
            .1 = quality;
    }

    /// Use discount rate to decide between rng and using the qable
    fn pick_action(&self, q_state: &QState) -> Action {
        if SmallRng::from_entropy().gen_range(0.0..1.0) < self.discount_rate {
            Action::random()
        } else {
            self.get_best_action(q_state).0
        }
    }

    fn get_best_action(&self, q_state: &QState) -> (Action, f64) {
        match self.q_table.get(q_state) {
            Some(actions) => *actions
                .iter()
                .max_by(|(_, q1), (_, q2)| q1.total_cmp(q2))
                .unwrap(),
            None => (Action::random(), DEFAULT_QUALITY),
        }
    }

    fn tile_at_coord(board: &Board, coord: (usize, usize)) -> Tile {
        if board.ghosts.contains(&coord) {
            Tile::Ghost
        } else {
            match board.tiles[coord.1][coord.0] {
                Background::Empty => Tile::Empty,
                Background::Wall => Tile::Wall,
            }
        }
    }

    fn make_q_state(board: &Board) -> QState {
        let (xp, yp) = board.pacman;
        let x = xp as i64;
        let y = yp as i64;
        // Gnarly but gets the job done
        let mut surroundings = [Tile::Empty; 24];
        let mut i = 0;
        let height = board.tiles.len() as i64;
        let width = board.tiles[0].len() as i64;
        for yi in y - 2..=y + 2 {
            for xi in x - 2..=x + 2 {
                if xi == x && yi == y {
                    continue;
                }
                if xi >= 0 && yi >= 0 && xi < width && yi < height {
                    surroundings[i] = PaQman::tile_at_coord(board, (xi as usize, yi as usize));
                }
                i += 1;
            }
        }
        // TODO: Try to figure out how to convert gx & gy into surroundings indices
        for (gx, gy) in board.ghosts.iter() {}
        QState { surroundings }
    }
}

impl PacmanAi for PaQman {
    fn tick_pacman(&mut self, board: &mut Board) {
        let current_state = PaQman::make_q_state(board);
        let action = self.pick_action(&current_state);
        let (x, y) = board.pacman;
        let new_coords = match action {
            Action::Idle => (x, y),
            Action::Up => (x, y - 1),
            Action::Down => (x, y + 1),
            Action::Left => (x - 1, y),
            Action::Right => (x + 1, y),
        };
        board.pacman = new_coords;
        let reward = if board.is_game_over() { -1.0 } else { 0.0 };
        self.update_q_table(reward, &current_state, &PaQman::make_q_state(board), action);
    }
}

#[derive(Parser, Debug)]
struct Opts {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Train {
        #[arg(short, long, default_value = "100000")]
        iterations: usize,
        #[arg(short, long)]
        q_path: String,
    },
    Run {
        #[arg(short, long)]
        q_path: String,
    },
}

fn run_train(iterations: usize, q_path: String) {
    let mut ai = PaQman::new();
    for iter in 0..iterations {
        let mut board = Board::initialize(10, 10, 7);
        while !board.is_game_over() {
            ai.tick_pacman(&mut board);
            if board.is_game_over() {
                break;
            }
            board.tick_ghosts();
        }
        if iter % 1000 == 0 {
            print!("Ran iteration {iter}, {}\r", ai.discount_rate);
            stdout().flush().expect("I couldn't flush the toilet :(");
        }
        let min_epsilon = 0.05f64;
        let r = -min_epsilon.ln() / iterations as f64;
        ai.discount_rate = (-r * iter as f64).exp();
    }
    let contents = to_allocvec(&ai.q_table).unwrap();
    let mut file = File::create(q_path).unwrap();
    file.write_all(&contents).unwrap();
}

fn run_game(q_path: String) {
    let mut ai = PaQman::new();

    let bytes = fs::read(q_path).unwrap();
    ai.q_table = from_bytes(&bytes).unwrap();
    ai.discount_rate = 0.0;

    let mut board = Board::initialize(10, 10, 7);
    loop {
        board.render();
        sleep(Duration::from_millis(100));
        ai.tick_pacman(&mut board);
        if board.is_game_over() {
            println!("Game over... LOSER");
            break;
        }
        board.tick_ghosts();
        if board.is_game_over() {
            println!("Game over... LOSER");
            break;
        }
    }
}

fn main() {
    match Opts::parse().command {
        Command::Train { iterations, q_path } => {
            run_train(iterations, q_path);
        },
        Command::Run { q_path } => {
            run_game(q_path);
        },
    }
}
