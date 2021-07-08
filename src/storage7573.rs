pub struct Storage128(u128);
impl From<u128> for Storage128 {
    fn from(v: u128) -> Self { Self(v) }
}
impl Into<u128> for Storage128 {
    fn into(self) -> u128 { self.0 }
}

impl Storage128 {
    pub fn id_u8<'a>(&'a self) -> &'a u8 {
        unsafe { std::mem::transmute(&self.0) }
    }
    pub fn id_u8_mut<'a>(&'a mut self) -> &'a mut u8 {
        unsafe { std::mem::transmute(&mut self.0) }
    }
    pub fn storage_u8<'a>(&'a self) -> &'a u8 {
        unsafe { std::mem::transmute(std::mem::transmute::<&u128, usize>(&self.0) + 1) }
    }
    pub fn storage_u8_mut<'a>(&'a mut self) -> &'a mut u8 {
        unsafe { std::mem::transmute(std::mem::transmute::<&mut u128, usize>(&mut self.0) + 1) }
    }
    pub fn storage_u16<'a>(&'a self) -> &'a u16 {
        unsafe { std::mem::transmute(std::mem::transmute::<&u128, usize>(&self.0) + 1) }
    }
    pub fn storage_u16_mut<'a>(&'a mut self) -> &'a mut u16 {
        unsafe { std::mem::transmute(std::mem::transmute::<&mut u128, usize>(&mut self.0) + 1) }
    }
}

#[derive(Debug)]
pub enum Storage7573 {
    Planet(u8),
    PartOfPlayer(u16),
    Invalid,
}
const ID_PLANET: u8 = 1;
const ID_PART_OF_PLAYER: u8 = 2;
impl From<u128> for Storage7573 {
    fn from(v: u128) -> Self {
        let storage: Storage128 = v.into();
        match *storage.id_u8() {
            ID_PLANET => {
                Self::Planet(*storage.storage_u8())
            },
            ID_PART_OF_PLAYER => {
                Self::PartOfPlayer(*storage.storage_u16())
            },
            _ => Self::Invalid,
        }
    }
}
impl Into<u128> for Storage7573 {
    fn into(self) -> u128 {
        let mut storage: Storage128 = 0u128.into();
        match self {
            Self::Planet(planet_id) => {
                *storage.id_u8_mut() = ID_PLANET;
                *storage.storage_u8_mut() = planet_id;
            },
            Self::PartOfPlayer(player_id) => {
                *storage.id_u8_mut() = ID_PART_OF_PLAYER;
                *storage.storage_u16_mut() = player_id;
            },
            Self::Invalid => panic!(),
        };
        storage.into()
    }
}
impl Storage7573 {
    pub fn planet_id(&self) -> u8 {
        match self {
            Storage7573::Planet(id) => *id,
            _ => panic!("planet_id called on non storage7573::planet")
        }
    }
}
