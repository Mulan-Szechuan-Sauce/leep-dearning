use rand::{
    prelude::{SliceRandom, ThreadRng},
    Rng,
};
use std::{collections::HashMap, thread::sleep, time::Duration};

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
    rng: ThreadRng,
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
            rng: rand::thread_rng(),
        }
    }

    fn empty_with_edge_walls(width: usize, height: usize) -> Self {
        let mut board = Board::new(width, height);
        for x in 0..width {
            board.tiles[0][x] = Background::Wall;
            board.tiles[height - 1][x] = Background::Wall;
        }
        for y in 1..(height - 1) {
            board.tiles[y][0] = Background::Wall;
            board.tiles[y][width - 1] = Background::Wall;
        }
        board
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
            let available_moves = [(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)]
                .into_iter()
                .filter(|(x, y)| {
                    matches!(self.tiles[*y][*x], Background::Empty) && !self.has_ghost_at(*x, *y)
                })
                .collect::<Vec<_>>();
            if let Some((xp, yp)) = available_moves.choose(&mut self.rng) {
                self.ghosts[i] = (*xp, *yp);
            }
        }
    }

    fn is_game_over(&self) -> bool {
        let (x, y) = self.pacman;
        self.tiles[y][x] == Background::Wall || self.ghosts.contains(&self.pacman)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Action {
    Idle,
    Up,
    Down,
    Left,
    Right,
}

impl Action {
    fn random() -> Action {
        let mut rng = rand::thread_rng();
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

#[derive(Hash, PartialEq, Eq, Clone)]
enum Tile {
    Empty,
    Ghost,
    Wall,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct QState {
    surroundings: [Tile; 8],
}

struct PaQman {
    q_table: HashMap<QState, StateActions>,
    learning_rate: f64,
    /// Changes over time - odds that we'll decide to take a random action
    /// Between [0.0, 1.0]
    discount_rate: f64,
}

const DEFAULT_QUALITY: f64 = 0.0;

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
        if rand::thread_rng().gen_range(0.0..1.0) < self.discount_rate {
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
            if coord.1 >= board.tiles.len() || coord.0 >= board.tiles[0].len() {
                return Tile::Empty;
            }
            match board.tiles[coord.1][coord.0] {
                Background::Empty => Tile::Empty,
                Background::Wall => Tile::Wall,
            }
        }
    }

    fn make_q_state(board: &Board) -> QState {
        let (x, y) = board.pacman;
        // Gnarly but gets the job done
        let surroundings = [
            (x.wrapping_sub(1), y.wrapping_sub(1)),
            (x.wrapping_sub(1), y),
            (x.wrapping_sub(1), y+1),
            (x+1, y.wrapping_sub(1)),
            (x+1, y),
            (x+1, y+1),
            (x, y+1),
            (x, y.wrapping_sub(1)),
        ].map(|coord| PaQman::tile_at_coord(board, coord));
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
            Action::Up => (x, y-1),
            Action::Down => (x, y+1),
            Action::Left => (x-1, y),
            Action::Right => (x+1, y),
        };
        board.pacman = new_coords;
        let reward = if board.is_game_over() {
            -1.0
        } else {
            0.0
        };
        self.update_q_table(reward, &current_state, &PaQman::make_q_state(board), action);
        // TODO: Use the proper rate for this
        self.discount_rate *= 0.999995;

        // fn do_tick(&mut self, q_state: &QState) {
        //     let action = self.pick_action(q_state);
        //     // let outcome = board.make_move(action)
        //     self.update_q_table(todo!(), todo!(), todo!(), todo!())
        // }
    }
}

fn main() {
    let mut ai = PaQman::new();
    for iter in 0..100_000 {
        let mut board = Board::empty_with_edge_walls(10, 10);
        for x in 1..5 {
            board.ghosts.push((x, 1));
            board.ghosts.push((x, 2));
            board.ghosts.push((x, 3));
        }
        while !board.is_game_over() {
            ai.tick_pacman(&mut board);
            if board.is_game_over() {
                break;
            }
            board.tick_ghosts();
        }
        if iter % 1000 == 0 {
            println!("Ran iteration {iter}, {}", ai.discount_rate);
        }
    }
    ai.discount_rate = 0.0;

    let mut board = Board::empty_with_edge_walls(10, 10);
    for x in 1..5 {
        board.ghosts.push((x, 1));
        board.ghosts.push((x, 2));
        board.ghosts.push((x, 3));
    }
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
