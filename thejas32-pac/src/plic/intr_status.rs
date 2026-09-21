#[doc = "Register `INTR_STATUS` reader"]
pub type R = crate::R<IntrStatusSpec>;
#[doc = "Field `UART0` reader - UART0 interrupt"]
pub type Uart0R = crate::BitReader;
#[doc = "Field `UART1` reader - UART1 interrupt"]
pub type Uart1R = crate::BitReader;
#[doc = "Field `UART2` reader - UART2 interrupt"]
pub type Uart2R = crate::BitReader;
#[doc = "Field `SPI0` reader - SPI0 interrupt"]
pub type Spi0R = crate::BitReader;
#[doc = "Field `SPI1` reader - SPI1 interrupt"]
pub type Spi1R = crate::BitReader;
#[doc = "Field `I2C0` reader - I2C0 interrupt"]
pub type I2c0R = crate::BitReader;
#[doc = "Field `I2C1` reader - I2C1 interrupt"]
pub type I2c1R = crate::BitReader;
#[doc = "Field `TIMER0` reader - TIMER0 interrupt"]
pub type Timer0R = crate::BitReader;
#[doc = "Field `TIMER1` reader - TIMER1 interrupt"]
pub type Timer1R = crate::BitReader;
#[doc = "Field `TIMER2` reader - TIMER2 interrupt"]
pub type Timer2R = crate::BitReader;
#[doc = "Field `GPIO0` reader - GPIO0 interrupt"]
pub type Gpio0R = crate::BitReader;
#[doc = "Field `GPIO1` reader - GPIO1 interrupt"]
pub type Gpio1R = crate::BitReader;
#[doc = "Field `GPIO2` reader - GPIO2 interrupt"]
pub type Gpio2R = crate::BitReader;
#[doc = "Field `GPIO3` reader - GPIO3 interrupt"]
pub type Gpio3R = crate::BitReader;
#[doc = "Field `GPIO4` reader - GPIO4 interrupt"]
pub type Gpio4R = crate::BitReader;
#[doc = "Field `GPIO5` reader - GPIO5 interrupt"]
pub type Gpio5R = crate::BitReader;
#[doc = "Field `GPIO6` reader - GPIO6 interrupt"]
pub type Gpio6R = crate::BitReader;
#[doc = "Field `GPIO7` reader - GPIO7 interrupt"]
pub type Gpio7R = crate::BitReader;
#[doc = "Field `GPIO8` reader - GPIO8 interrupt"]
pub type Gpio8R = crate::BitReader;
#[doc = "Field `GPIO9` reader - GPIO9 interrupt"]
pub type Gpio9R = crate::BitReader;
#[doc = "Field `GPIO10` reader - GPIO10 interrupt"]
pub type Gpio10R = crate::BitReader;
#[doc = "Field `GPIO11` reader - GPIO11 interrupt"]
pub type Gpio11R = crate::BitReader;
#[doc = "Field `GPIO12` reader - GPIO12 interrupt"]
pub type Gpio12R = crate::BitReader;
impl R {
    #[doc = "Bit 0 - UART0 interrupt"]
    #[inline(always)]
    pub fn uart0(&self) -> Uart0R {
        Uart0R::new((self.bits & 1) != 0)
    }
    #[doc = "Bit 1 - UART1 interrupt"]
    #[inline(always)]
    pub fn uart1(&self) -> Uart1R {
        Uart1R::new(((self.bits >> 1) & 1) != 0)
    }
    #[doc = "Bit 2 - UART2 interrupt"]
    #[inline(always)]
    pub fn uart2(&self) -> Uart2R {
        Uart2R::new(((self.bits >> 2) & 1) != 0)
    }
    #[doc = "Bit 3 - SPI0 interrupt"]
    #[inline(always)]
    pub fn spi0(&self) -> Spi0R {
        Spi0R::new(((self.bits >> 3) & 1) != 0)
    }
    #[doc = "Bit 4 - SPI1 interrupt"]
    #[inline(always)]
    pub fn spi1(&self) -> Spi1R {
        Spi1R::new(((self.bits >> 4) & 1) != 0)
    }
    #[doc = "Bit 5 - I2C0 interrupt"]
    #[inline(always)]
    pub fn i2c0(&self) -> I2c0R {
        I2c0R::new(((self.bits >> 5) & 1) != 0)
    }
    #[doc = "Bit 6 - I2C1 interrupt"]
    #[inline(always)]
    pub fn i2c1(&self) -> I2c1R {
        I2c1R::new(((self.bits >> 6) & 1) != 0)
    }
    #[doc = "Bit 7 - TIMER0 interrupt"]
    #[inline(always)]
    pub fn timer0(&self) -> Timer0R {
        Timer0R::new(((self.bits >> 7) & 1) != 0)
    }
    #[doc = "Bit 8 - TIMER1 interrupt"]
    #[inline(always)]
    pub fn timer1(&self) -> Timer1R {
        Timer1R::new(((self.bits >> 8) & 1) != 0)
    }
    #[doc = "Bit 9 - TIMER2 interrupt"]
    #[inline(always)]
    pub fn timer2(&self) -> Timer2R {
        Timer2R::new(((self.bits >> 9) & 1) != 0)
    }
    #[doc = "Bit 10 - GPIO0 interrupt"]
    #[inline(always)]
    pub fn gpio0(&self) -> Gpio0R {
        Gpio0R::new(((self.bits >> 10) & 1) != 0)
    }
    #[doc = "Bit 11 - GPIO1 interrupt"]
    #[inline(always)]
    pub fn gpio1(&self) -> Gpio1R {
        Gpio1R::new(((self.bits >> 11) & 1) != 0)
    }
    #[doc = "Bit 12 - GPIO2 interrupt"]
    #[inline(always)]
    pub fn gpio2(&self) -> Gpio2R {
        Gpio2R::new(((self.bits >> 12) & 1) != 0)
    }
    #[doc = "Bit 13 - GPIO3 interrupt"]
    #[inline(always)]
    pub fn gpio3(&self) -> Gpio3R {
        Gpio3R::new(((self.bits >> 13) & 1) != 0)
    }
    #[doc = "Bit 14 - GPIO4 interrupt"]
    #[inline(always)]
    pub fn gpio4(&self) -> Gpio4R {
        Gpio4R::new(((self.bits >> 14) & 1) != 0)
    }
    #[doc = "Bit 15 - GPIO5 interrupt"]
    #[inline(always)]
    pub fn gpio5(&self) -> Gpio5R {
        Gpio5R::new(((self.bits >> 15) & 1) != 0)
    }
    #[doc = "Bit 16 - GPIO6 interrupt"]
    #[inline(always)]
    pub fn gpio6(&self) -> Gpio6R {
        Gpio6R::new(((self.bits >> 16) & 1) != 0)
    }
    #[doc = "Bit 17 - GPIO7 interrupt"]
    #[inline(always)]
    pub fn gpio7(&self) -> Gpio7R {
        Gpio7R::new(((self.bits >> 17) & 1) != 0)
    }
    #[doc = "Bit 18 - GPIO8 interrupt"]
    #[inline(always)]
    pub fn gpio8(&self) -> Gpio8R {
        Gpio8R::new(((self.bits >> 18) & 1) != 0)
    }
    #[doc = "Bit 19 - GPIO9 interrupt"]
    #[inline(always)]
    pub fn gpio9(&self) -> Gpio9R {
        Gpio9R::new(((self.bits >> 19) & 1) != 0)
    }
    #[doc = "Bit 20 - GPIO10 interrupt"]
    #[inline(always)]
    pub fn gpio10(&self) -> Gpio10R {
        Gpio10R::new(((self.bits >> 20) & 1) != 0)
    }
    #[doc = "Bit 21 - GPIO11 interrupt"]
    #[inline(always)]
    pub fn gpio11(&self) -> Gpio11R {
        Gpio11R::new(((self.bits >> 21) & 1) != 0)
    }
    #[doc = "Bit 22 - GPIO12 interrupt"]
    #[inline(always)]
    pub fn gpio12(&self) -> Gpio12R {
        Gpio12R::new(((self.bits >> 22) & 1) != 0)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("INTR_STATUS")
            .field("uart0", &self.uart0())
            .field("uart1", &self.uart1())
            .field("uart2", &self.uart2())
            .field("spi0", &self.spi0())
            .field("spi1", &self.spi1())
            .field("i2c0", &self.i2c0())
            .field("i2c1", &self.i2c1())
            .field("timer0", &self.timer0())
            .field("timer1", &self.timer1())
            .field("timer2", &self.timer2())
            .field("gpio0", &self.gpio0())
            .field("gpio1", &self.gpio1())
            .field("gpio2", &self.gpio2())
            .field("gpio3", &self.gpio3())
            .field("gpio4", &self.gpio4())
            .field("gpio5", &self.gpio5())
            .field("gpio6", &self.gpio6())
            .field("gpio7", &self.gpio7())
            .field("gpio8", &self.gpio8())
            .field("gpio9", &self.gpio9())
            .field("gpio10", &self.gpio10())
            .field("gpio11", &self.gpio11())
            .field("gpio12", &self.gpio12())
            .finish()
    }
}
#[doc = "Masked interrupt status\n\nYou can [`read`](crate::Reg::read) this register and get [`intr_status::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct IntrStatusSpec;
impl crate::RegisterSpec for IntrStatusSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`intr_status::R`](R) reader structure"]
impl crate::Readable for IntrStatusSpec {}
#[doc = "`reset()` method sets INTR_STATUS to value 0"]
impl crate::Resettable for IntrStatusSpec {}
