#[doc = r"Enumeration of all the interrupts."]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Interrupt {
    #[doc = "0 - UART0"]
    UART0 = 0,
    #[doc = "1 - UART1"]
    UART1 = 1,
    #[doc = "2 - UART2"]
    UART2 = 2,
    #[doc = "3 - SPI0"]
    SPI0 = 3,
    #[doc = "4 - SPI1"]
    SPI1 = 4,
    #[doc = "5 - I2C0"]
    I2C0 = 5,
    #[doc = "6 - I2C1"]
    I2C1 = 6,
    #[doc = "7 - TIMER0"]
    TIMER0 = 7,
    #[doc = "8 - TIMER1"]
    TIMER1 = 8,
    #[doc = "9 - TIMER2"]
    TIMER2 = 9,
    #[doc = "10 - GPIO0"]
    GPIO0 = 10,
    #[doc = "11 - GPIO1"]
    GPIO1 = 11,
    #[doc = "12 - GPIO2"]
    GPIO2 = 12,
    #[doc = "13 - GPIO3"]
    GPIO3 = 13,
    #[doc = "14 - GPIO4"]
    GPIO4 = 14,
    #[doc = "15 - GPIO5"]
    GPIO5 = 15,
    #[doc = "16 - GPIO6"]
    GPIO6 = 16,
    #[doc = "17 - GPIO7"]
    GPIO7 = 17,
    #[doc = "18 - GPIO8"]
    GPIO8 = 18,
    #[doc = "19 - GPIO9"]
    GPIO9 = 19,
    #[doc = "20 - GPIO10"]
    GPIO10 = 20,
    #[doc = "21 - GPIO11"]
    GPIO11 = 21,
    #[doc = "22 - GPIO12"]
    GPIO12 = 22,
}
#[doc = r" TryFromInterruptError"]
#[derive(Debug, Copy, Clone)]
pub struct TryFromInterruptError(());
impl Interrupt {
    #[doc = r" Attempt to convert a given value into an `Interrupt`"]
    #[inline]
    pub fn try_from(value: u8) -> Result<Self, TryFromInterruptError> {
        match value {
            0 => Ok(Interrupt::UART0),
            1 => Ok(Interrupt::UART1),
            2 => Ok(Interrupt::UART2),
            3 => Ok(Interrupt::SPI0),
            4 => Ok(Interrupt::SPI1),
            5 => Ok(Interrupt::I2C0),
            6 => Ok(Interrupt::I2C1),
            7 => Ok(Interrupt::TIMER0),
            8 => Ok(Interrupt::TIMER1),
            9 => Ok(Interrupt::TIMER2),
            10 => Ok(Interrupt::GPIO0),
            11 => Ok(Interrupt::GPIO1),
            12 => Ok(Interrupt::GPIO2),
            13 => Ok(Interrupt::GPIO3),
            14 => Ok(Interrupt::GPIO4),
            15 => Ok(Interrupt::GPIO5),
            16 => Ok(Interrupt::GPIO6),
            17 => Ok(Interrupt::GPIO7),
            18 => Ok(Interrupt::GPIO8),
            19 => Ok(Interrupt::GPIO9),
            20 => Ok(Interrupt::GPIO10),
            21 => Ok(Interrupt::GPIO11),
            22 => Ok(Interrupt::GPIO12),
            _ => Err(TryFromInterruptError(())),
        }
    }
}
#[cfg(feature = "rt")]
#[macro_export]
#[doc = r" Assigns a handler to an interrupt"]
#[doc = r""]
#[doc = r" This macro takes two arguments: the name of an interrupt and the path to the"]
#[doc = r" function that will be used as the handler of that interrupt. That function"]
#[doc = r" must have signature `fn()`."]
#[doc = r""]
#[doc = r" Optionally, a third argument may be used to declare interrupt local data."]
#[doc = r" The handler will have exclusive access to these *local* variables on each"]
#[doc = r" invocation. If the third argument is used then the signature of the handler"]
#[doc = r" function must be `fn(&mut $NAME::Locals)` where `$NAME` is the first argument"]
#[doc = r" passed to the macro."]
#[doc = r""]
#[doc = r" # Example"]
#[doc = r""]
#[doc = r" ``` ignore"]
#[doc = r" interrupt!(TIM2, periodic);"]
#[doc = r""]
#[doc = r" fn periodic() {"]
#[doc = r#"     print!(".");"#]
#[doc = r" }"]
#[doc = r""]
#[doc = r" interrupt!(TIM3, tick, locals: {"]
#[doc = r"     tick: bool = false;"]
#[doc = r" });"]
#[doc = r""]
#[doc = r" fn tick(locals: &mut TIM3::Locals) {"]
#[doc = r"     locals.tick = !locals.tick;"]
#[doc = r""]
#[doc = r"     if locals.tick {"]
#[doc = r#"         println!("Tick");"#]
#[doc = r"     } else {"]
#[doc = r#"         println!("Tock");"#]
#[doc = r"     }"]
#[doc = r" }"]
#[doc = r" ```"]
macro_rules ! interrupt { ($ NAME : ident , $ path : path , locals : { $ ($ lvar : ident : $ lty : ty = $ lval : expr ;) * }) => { # [allow (non_snake_case)] mod $ NAME { pub struct Locals { $ (pub $ lvar : $ lty ,) * } } # [allow (non_snake_case)] # [no_mangle] pub extern "C" fn $ NAME () { let _ = $ crate :: interrupt :: Interrupt :: $ NAME ; static mut LOCALS : self :: $ NAME :: Locals = self :: $ NAME :: Locals { $ ($ lvar : $ lval ,) * } ; let f : fn (& mut self :: $ NAME :: Locals) = $ path ; f (unsafe { & mut LOCALS }) ; } } ; ($ NAME : ident , $ path : path) => { # [allow (non_snake_case)] # [no_mangle] pub extern "C" fn $ NAME () { let _ = $ crate :: interrupt :: Interrupt :: $ NAME ; let f : fn () = $ path ; f () ; } } }
