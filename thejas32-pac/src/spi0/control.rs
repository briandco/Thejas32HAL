#[doc = "Register `CONTROL` reader"]
pub type R = crate::R<ControlSpec>;
#[doc = "Register `CONTROL` writer"]
pub type W = crate::W<ControlSpec>;
#[doc = "Field `PERIPH_CS` reader - Peripheral chip select"]
pub type PeriphCsR = crate::FieldReader;
#[doc = "Field `PERIPH_CS` writer - Peripheral chip select"]
pub type PeriphCsW<'a, REG> = crate::FieldWriter<'a, REG, 2>;
#[doc = "Field `PERIPH` reader - Peripheral type (0=fixed, 1=variable)"]
pub type PeriphR = crate::BitReader;
#[doc = "Field `PERIPH` writer - Peripheral type (0=fixed, 1=variable)"]
pub type PeriphW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `DATA_ORDER` reader - Data order (0=MSB first, 1=LSB first)"]
pub type DataOrderR = crate::BitReader;
#[doc = "Field `DATA_ORDER` writer - Data order (0=MSB first, 1=LSB first)"]
pub type DataOrderW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `MODE` reader - SPI mode (clock polarity/phase)"]
pub type ModeR = crate::FieldReader;
#[doc = "Field `MODE` writer - SPI mode (clock polarity/phase)"]
pub type ModeW<'a, REG> = crate::FieldWriter<'a, REG, 2>;
#[doc = "Field `RX_TX_INTR` reader - RX/TX interrupt enable"]
pub type RxTxIntrR = crate::FieldReader;
#[doc = "Field `RX_TX_INTR` writer - RX/TX interrupt enable"]
pub type RxTxIntrW<'a, REG> = crate::FieldWriter<'a, REG, 2>;
#[doc = "Field `CSAAT` reader - Chip select active after transfer"]
pub type CsaatR = crate::BitReader;
#[doc = "Field `CSAAT` writer - Chip select active after transfer"]
pub type CsaatW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `DBITS` reader - Data bits per transfer"]
pub type DbitsR = crate::FieldReader;
#[doc = "Field `DBITS` writer - Data bits per transfer"]
pub type DbitsW<'a, REG> = crate::FieldWriter<'a, REG, 4>;
impl R {
    #[doc = "Bits 0:1 - Peripheral chip select"]
    #[inline(always)]
    pub fn periph_cs(&self) -> PeriphCsR {
        PeriphCsR::new((self.bits & 3) as u8)
    }
    #[doc = "Bit 2 - Peripheral type (0=fixed, 1=variable)"]
    #[inline(always)]
    pub fn periph(&self) -> PeriphR {
        PeriphR::new(((self.bits >> 2) & 1) != 0)
    }
    #[doc = "Bit 3 - Data order (0=MSB first, 1=LSB first)"]
    #[inline(always)]
    pub fn data_order(&self) -> DataOrderR {
        DataOrderR::new(((self.bits >> 3) & 1) != 0)
    }
    #[doc = "Bits 4:5 - SPI mode (clock polarity/phase)"]
    #[inline(always)]
    pub fn mode(&self) -> ModeR {
        ModeR::new(((self.bits >> 4) & 3) as u8)
    }
    #[doc = "Bits 6:7 - RX/TX interrupt enable"]
    #[inline(always)]
    pub fn rx_tx_intr(&self) -> RxTxIntrR {
        RxTxIntrR::new(((self.bits >> 6) & 3) as u8)
    }
    #[doc = "Bit 8 - Chip select active after transfer"]
    #[inline(always)]
    pub fn csaat(&self) -> CsaatR {
        CsaatR::new(((self.bits >> 8) & 1) != 0)
    }
    #[doc = "Bits 9:12 - Data bits per transfer"]
    #[inline(always)]
    pub fn dbits(&self) -> DbitsR {
        DbitsR::new(((self.bits >> 9) & 0x0f) as u8)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("CONTROL")
            .field("periph_cs", &self.periph_cs())
            .field("periph", &self.periph())
            .field("data_order", &self.data_order())
            .field("mode", &self.mode())
            .field("rx_tx_intr", &self.rx_tx_intr())
            .field("csaat", &self.csaat())
            .field("dbits", &self.dbits())
            .finish()
    }
}
impl W {
    #[doc = "Bits 0:1 - Peripheral chip select"]
    #[inline(always)]
    pub fn periph_cs(&mut self) -> PeriphCsW<'_, ControlSpec> {
        PeriphCsW::new(self, 0)
    }
    #[doc = "Bit 2 - Peripheral type (0=fixed, 1=variable)"]
    #[inline(always)]
    pub fn periph(&mut self) -> PeriphW<'_, ControlSpec> {
        PeriphW::new(self, 2)
    }
    #[doc = "Bit 3 - Data order (0=MSB first, 1=LSB first)"]
    #[inline(always)]
    pub fn data_order(&mut self) -> DataOrderW<'_, ControlSpec> {
        DataOrderW::new(self, 3)
    }
    #[doc = "Bits 4:5 - SPI mode (clock polarity/phase)"]
    #[inline(always)]
    pub fn mode(&mut self) -> ModeW<'_, ControlSpec> {
        ModeW::new(self, 4)
    }
    #[doc = "Bits 6:7 - RX/TX interrupt enable"]
    #[inline(always)]
    pub fn rx_tx_intr(&mut self) -> RxTxIntrW<'_, ControlSpec> {
        RxTxIntrW::new(self, 6)
    }
    #[doc = "Bit 8 - Chip select active after transfer"]
    #[inline(always)]
    pub fn csaat(&mut self) -> CsaatW<'_, ControlSpec> {
        CsaatW::new(self, 8)
    }
    #[doc = "Bits 9:12 - Data bits per transfer"]
    #[inline(always)]
    pub fn dbits(&mut self) -> DbitsW<'_, ControlSpec> {
        DbitsW::new(self, 9)
    }
}
#[doc = "SPI Control Register\n\nYou can [`read`](crate::Reg::read) this register and get [`control::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`control::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct ControlSpec;
impl crate::RegisterSpec for ControlSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`control::R`](R) reader structure"]
impl crate::Readable for ControlSpec {}
#[doc = "`write(|w| ..)` method takes [`control::W`](W) writer structure"]
impl crate::Writable for ControlSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets CONTROL to value 0"]
impl crate::Resettable for ControlSpec {}
