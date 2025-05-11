use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use rand::Rng;
use crossterm::{
    ExecutableCommand, cursor, terminal,
    event::{self, Event, KeyCode}
};

const WIDTH: u16 = 40;
const HEIGHT: u16 = 20;
const FRAME_DURATION: Duration = Duration::from_millis(300);
const FOOD_COUNT: usize = 15;
const WIN_SCORE: u32 = 20;
const TIME_LIMIT: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq)]
struct Point {
    x: u16,
    y: u16,
}

struct Snake {
    body: Vec<Point>,
    direction: Direction,
}

impl Snake {
    fn new(start_x: u16, start_y: u16) -> Snake {
        Snake {
            body: vec![Point { x: start_x, y: start_y }],
            direction: Direction::Right,
        }
    }

    fn move_forward(&mut self) {
        let head = *self.body.first().expect("Snake has no body");
        let new_head = match self.direction {
            Direction::Up => Point { x: head.x, y: head.y.saturating_sub(1) },
            Direction::Down => Point { x: head.x, y: head.y.saturating_add(1) },
            Direction::Left => Point { x: head.x.saturating_sub(1), y: head.y },
            Direction::Right => Point { x: head.x.saturating_add(1), y: head.y },
        };

        self.body.insert(0, new_head);
        self.body.pop();
    }

    fn grow(&mut self) {
        let tail = *self.body.last().expect("Snake has no body");
        self.body.push(tail);
    }

    fn change_direction(&mut self, new_direction: Direction) {
        match self.direction {
            Direction::Up if new_direction != Direction::Down => self.direction = new_direction,
            Direction::Down if new_direction != Direction::Up => self.direction = new_direction,
            Direction::Left if new_direction != Direction::Right => self.direction = new_direction,
            Direction::Right if new_direction != Direction::Left => self.direction = new_direction,
            _ => {}
        }
    }
}

struct Game {
    snake: Snake,
    food: Vec<Point>,
    score: u32,
    game_over: bool,
    game_won: bool,
    paused: bool,
    start_time: Instant, 
    pause_start_time: Option<Instant>, 
    total_pause_duration: Duration, 
    game_over_message: Option<String>,
}

impl Game {
    fn new() -> Game {
        let mut rng = rand::thread_rng();
        let mut food = Vec::new();

        for _ in 0..FOOD_COUNT {
            let food_x = rng.gen_range(1..WIDTH);
            let food_y = rng.gen_range(1..HEIGHT);
            food.push(Point { x: food_x, y: food_y });
        }

        Game {
            snake: Snake::new(WIDTH / 2, HEIGHT / 2),
            food,
            score: 0,
            game_over: false,
            game_won: false,
            paused: false,
            start_time: Instant::now(),
            pause_start_time: None,
            total_pause_duration: Duration::new(0, 0),
            game_over_message: None,
        }
    }

    fn update(&mut self) {
        if self.game_over || self.paused || self.game_won {
            return;
        }

        let elapsed_time = self.start_time.elapsed() - self.total_pause_duration;
        if elapsed_time >= TIME_LIMIT {
            self.game_over = true;
            self.game_over_message = Some("You ran out of time!".to_string());
            return;
        }

        self.snake.move_forward();

        let head = *self.snake.body.first().expect("Snake has no head");


        if head.x == 0 || head.y == 0 || head.x == WIDTH || head.y == HEIGHT {
            self.game_over = true;
            self.game_over_message = Some("You hit the wall!".to_string());
        } else if self.snake.body[1..].contains(&head) {
            self.game_over = true;
            self.game_over_message = Some("You hit yourself!".to_string());
        }


        if let Some(index) = self.food.iter().position(|&food| food == head) {
            self.snake.grow();
            self.score += 1;
            self.food.remove(index);
            self.generate_food();

            if self.score >= WIN_SCORE {
                self.game_won = true;
            }
        }
    }

    fn generate_food(&mut self) {
        let mut rng = rand::thread_rng();
        let new_food = loop {
            let new_food = Point {
                x: rng.gen_range(1..WIDTH),
                y: rng.gen_range(1..HEIGHT),
            };
            if !self.is_snake(new_food.x, new_food.y) && !self.is_food(new_food.x, new_food.y) {
                break new_food;
            }
        };
        self.food.push(new_food);
    }

    fn draw(&self) {
        stdout().execute(terminal::Clear(terminal::ClearType::All)).unwrap();

        for y in 0..=HEIGHT {
            for x in 0..=WIDTH {
                if self.is_snake(x, y) {
                    print!("O");
                } else if self.is_food(x, y) {
                    print!("*");
                } else if x == 0 || y == 0 || x == WIDTH || y == HEIGHT {
                    print!("#");
                } else {
                    print!(" ");
                }
            }
            println!();
        }

        let elapsed_time = if self.paused {
            self.pause_start_time.unwrap().duration_since(self.start_time) - self.total_pause_duration
        } else {
            self.start_time.elapsed() - self.total_pause_duration
        };

        let remaining_time = if elapsed_time >= TIME_LIMIT {
            Duration::from_secs(0)
        } else {
            TIME_LIMIT - elapsed_time
        };

        println!("Score: {}", self.score);
        println!("Time left: {:02}:{:02}", remaining_time.as_secs() / 60, remaining_time.as_secs() % 60);

        if self.paused {
            println!("Game Paused. Press 'p' to resume.");
        }

        if self.game_won {
            println!("Congratulations! You've won the game with a score of {}!", self.score);
        }

        stdout().flush().unwrap();
    }

    fn toggle_pause(&mut self) {
        if self.paused {
            if let Some(pause_start_time) = self.pause_start_time {
                self.total_pause_duration += pause_start_time.elapsed();
            }
            self.paused = false;
            self.pause_start_time = None;
        } else {
            self.paused = true;
            self.pause_start_time = Some(Instant::now());
        }
    }

    fn is_snake(&self, x: u16, y: u16) -> bool {
        self.snake.body.iter().any(|&point| point.x == x && point.y == y)
    }

    fn is_food(&self, x: u16, y: u16) -> bool {
        self.food.iter().any(|&point| point.x == x && point.y == y)
    }
}

fn main() {
    loop {
        let mut stdout = stdout();
        stdout.execute(terminal::EnterAlternateScreen).unwrap();
        stdout.execute(cursor::Hide).unwrap();
        terminal::enable_raw_mode().unwrap();

        let (tx, rx): (Sender<Direction>, Receiver<Direction>) = mpsc::channel();

        thread::spawn(move || {
            loop {
                if let Event::Key(event) = event::read().unwrap() {
                    let direction = match event.code {
                        KeyCode::Up => Direction::Up,
                        KeyCode::Down => Direction::Down,
                        KeyCode::Left => Direction::Left,
                        KeyCode::Right => Direction::Right,
                        _ => continue,
                    };
                    tx.send(direction).unwrap();
                }
            }
        });

        let mut game = Game::new();
        let mut last_update = Instant::now();

        loop {
            if event::poll(Duration::from_millis(10)).unwrap() {
                if let Event::Key(event) = event::read().unwrap() {
                    match event.code {
                        KeyCode::Esc => break,
                        KeyCode::Char('p') => game.toggle_pause(),
                        _ => (),
                    }
                }
            }

            if let Ok(direction) = rx.try_recv() {
                game.snake.change_direction(direction);
            }

            if last_update.elapsed() >= FRAME_DURATION {
                game.update();
                game.draw();
                last_update = Instant::now();
            }

            if game.game_over || game.game_won {
                break;
            }
        }

        terminal::disable_raw_mode().unwrap();
        stdout.execute(cursor::Show).unwrap();
        stdout.execute(terminal::LeaveAlternateScreen).unwrap();

        if game.game_won {
            println!("Congratulations! You've won the game with a score of {}!", game.score);
        } else if let Some(message) = &game.game_over_message {
            println!("Game Over! {} Your final score was: {}", message, game.score);
        }

        println!("Press 'r' to retry or Enter to exit...");
        
        let mut retry = false;
        while let Event::Key(event) = event::read().unwrap() {
            match event.code {
                KeyCode::Char('r') => {
                    retry = true;
                    break;
                }
                KeyCode::Enter => break,
                _ => (),
            }
        }
        
        if !retry {
            break;
        }
    }

    println!("Thanks for playing!");
}
