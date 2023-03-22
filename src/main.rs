use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::collections::HashMap;

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
struct Vec2([i8; 2]);

impl Vec2 {
    fn new(x: i8, y: i8) -> Vec2 {
        Vec2([x, y])
    }
    fn flip(&self) -> Vec2 {
        Vec2([-self.0[0], -self.0[1]])
    }
}

#[derive(PartialEq, Clone)]
struct Rule {
    // The 'WEIGHTAGE' / how likely is it that this cell will spawn
    weight: f32,
    // The cell the rules is being defined for
    cell: CellType,
    // A hashmap of cells that are allowed to be around this cell
    // to the Vec of the directions it can be
    cell_map: HashMap<Vec2, Vec<CellType>>,
}

#[derive(PartialEq, Clone)]
struct Rules(Vec<Rule>);

impl Rules {
    fn get_rule(&self, cell: CellType) -> Option<&Rule> {
        return self.0.iter().filter(|r| r.cell == cell).next();
    }
    fn get_possibilities(&self, cell: CellType, direction: &Vec2) -> Vec<CellType> {
        self.0
            .iter()
            .filter(|r| r.cell == cell)
            .next()
            .unwrap()
            .cell_map
            .iter()
            .filter(|(dir, _)| *dir == direction)
            .map(|(_, cells)| (*cells).clone())
            .collect::<Vec<Vec<CellType>>>()
            .concat()
    }
}

impl Rule {
    fn new(cell: CellType, weight: f32) -> Rule {
        return Rule {
            weight,
            cell,
            cell_map: HashMap::new(),
        };
    }
    fn push_cell_set(&mut self, cell: CellType, direction: Vec2) {
        if self.cell_map.contains_key(&direction) {
            self.cell_map.get_mut(&direction).unwrap().push(cell);
        } else {
            self.cell_map.insert(direction, vec![cell]);
        }
    }
}
#[derive(Eq, PartialEq, Hash, Clone, Debug)]
enum CellType {
    // When the cell hasn't collapsed, it stores the types it can be
    Uncertain(Vec<CellType>),
    // When the cell has collapsed, it stores the type it was
    Space,
    //Gas,
    //Plasma,
    Core,
    // When Cell is initialized, it is of type None
    // It represents the state when the cell could be of any type
    None,
}

impl CellType {
    fn get_color(&self) -> image::Rgb<u8> {
        match self {
            CellType::None => image::Rgb::from([0, 0, 0]),
            CellType::Core => image::Rgb::from([230, 220, 200]),
            //CellType::Plasma => image::Rgb::from([240, 240, 240]),
            //CellType::Gas => image::Rgb::from([240, 240, 240]),
            CellType::Space => image::Rgb::from([20, 20, 50]),
            CellType::Uncertain(_) => image::Rgb::from([0, 0, 0]),
        }
    }
}

#[derive(PartialEq, Clone)]
struct Field {
    width: u32,
    height: u32,
    cells: Vec<CellType>,
    rules: Rules,
}

impl Field {
    fn new(w: u32, h: u32, rules: Rules) -> Field {
        return Field {
            width: w,
            height: h,
            cells: vec![CellType::None; (w * h) as usize],
            rules,
        };
    }
    fn get_color(&self, x: u32, y: u32) -> image::Rgb<u8> {
        self.cells[(self.width * y + x) as usize].get_color()
    }
    fn set(&mut self, x: u32, y: u32, tile: &CellType) {
        self.cells[(y * self.width + x) as usize] = tile.clone();
    }
    fn set_dir(&mut self, cx: u32, cy: u32, dir: &Vec2, tile: &CellType) {
        self.set(
            (cx as i32 + dir.0[0] as i32) as u32,
            (cy as i32 + dir.0[1] as i32) as u32,
            tile,
        );
    }
    fn observe(&mut self, x: u32, y: u32) {
        let tile = self.get(x, y);
        let binding = self.clone();
        println!("{:?}", tile);
        let rule = binding
            .rules
            .get_rule(tile)
            .expect("There is no rule matching this tile");
        for (direction, cells) in rule.cell_map.iter() {
            let mut uncertain_cell = CellType::Uncertain(vec![]);
            for cell in cells.iter() {
                println!("x: {}, y: {}, direction: {:?}", x, y, direction);
                match self.get_dir(x, y, direction) {
                    CellType::Uncertain(v) => {
                        if v.len() == 1 {
                            self.set_dir(x, y, direction, &v[0]);
                            return;
                        }
                        uncertain_cell = match uncertain_cell {
                            CellType::Uncertain(possibilities) => CellType::Uncertain(
                                possibilities
                                    .iter()
                                    .chain(&mut v.iter().filter(|c| *c == cell))
                                    .map(|c| c.clone())
                                    .collect(),
                            ),
                            _ => CellType::None,
                        };
                        self.set_dir(x, y, direction, &uncertain_cell);
                    }
                    CellType::None => {
                        for rule in binding.rules.0.iter() {
                            for direction in rule.cell_map.keys() {
                                match self.get_dir(x, y, direction) {
                                    CellType::None => {}
                                    x => {
                                        let possibilites =
                                            self.rules.get_possibilities(x, direction);
                                        uncertain_cell = match uncertain_cell {
                                            CellType::Uncertain(possibilities_union) => {
                                                CellType::Uncertain(
                                                    possibilities_union
                                                        .iter()
                                                        .chain(
                                                            &mut possibilites
                                                                .iter()
                                                                .filter(|c| *c == cell),
                                                        )
                                                        .map(|c| c.clone())
                                                        .collect(),
                                                )
                                            }
                                            _ => CellType::None,
                                        };
                                    }
                                }
                            }
                        }
                    }
                    _ => self.observe(
                        (x as i32 + direction.0[0] as i32) as u32,
                        (y as i32 + direction.0[1] as i32) as u32,
                    ),
                };
            }
        }
    }
    fn render(&self, dest: String) {
        let mut img = image::RgbImage::new(self.width, self.height);
        for x in 0..self.width {
            for y in 0..self.height {
                img.put_pixel(x, y, self.get_color(x, y))
            }
        }
    }
    fn to_u32_arr(&self) -> Vec<u32> {
        self.cells
            .iter()
            .map(|cell| {
                let color = cell.get_color();
                (color.0[2]) as u32 | (((color.0[1]) as u32) << 8) | ((color.0[0]) as u32) << 16
            })
            .collect()
    }
    fn get(&self, x: u32, y: u32) -> CellType {
        self.cells[(self.width * y + x) as usize].clone()
    }
    fn get_dir(&mut self, cx: u32, cy: u32, dir: &Vec2) -> CellType {
        self.get(
            (cx as i32 + dir.0[0] as i32) as u32,
            (cy as i32 + dir.0[1] as i32) as u32,
        )
    }
}

fn main() {
    let mut window = Window::new("Wave Function Collapse", 80, 80, WindowOptions::default())
        .expect("Should have opened a window");
    let mut rules = vec![];
    let mut rule = Rule::new(CellType::Core, 0.5);
    rule.push_cell_set(CellType::Space, Vec2::new(1, 1));
    rules.push(rule);
    let mut rule = Rule::new(CellType::Space, 0.5);
    rule.push_cell_set(CellType::Core, Vec2::new(-1, -1));
    rules.push(rule);
    let mut f = Field::new(80, 80, Rules(rules));
    f.set(50, 50, &CellType::Core);
    f.observe(50, 50);
    window.limit_update_rate(Some(std::time::Duration::from_millis(4)));
    let buf = f.to_u32_arr();
    while window.is_open() || !window.is_key_pressed(Key::Escape, KeyRepeat::Yes) {
        window.update_with_buffer(&buf, 80, 80).unwrap();
    }
}
