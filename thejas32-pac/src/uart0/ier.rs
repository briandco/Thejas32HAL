#[doc = "Register `IER` reader"]
pub type R = crate::R<IerSpec>;
#[doc = "Register `IER` writer"]
pub type W = crate::W<IerSpec>;
#[doc = "Field `ERBFI` reader - Enable received data available interrupt"]
pub type ErbfiR = crate::BitReader;
#[doc = "Field `ERBFI` writer - Enable received data available interrupt"]
pub type ErbfiW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `ETBEI` reader - Enable transmitter holding register empty interrupt"]
pub type EtbeiR = crate::BitReader;
#[doc = "Field `ETBEI` writer - Enable transmitter holding register empty interrupt"]
pub type EtbeiW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `ELSI` reader - Enable receiver line status interrupt"]
pub type ElsiR = crate::BitReader;
#[doc = "Field `ELSI` writer - Enable receiver line status interrupt"]
pub type ElsiW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `EDSSI` reader - Enable modem status interrupt"]
pub type EdssiR = crate::BitReader;
#[doc = "Field `EDSSI` writer - Enable modem status interrupt"]
pub type EdssiW<'a, REG> = crate::BitWriter<'a, REG>;
impl R {
    #[doc = "Bit 0 - Enable received data available interrupt"]
    #[inline(always)]
    pub fn erbfi(&self) -> ErbfiR {
        ErbfiR::new((self.bits & 1) != 0)
    }
    #[doc = "Bit 1 - Enable transmitter holding register empty interrupt"]
    #[inline(always)]
    pub fn etbei(&self) -> EtbeiR {
        EtbeiR::new(((self.bits >> 1) & 1) != 0)
    }
    #[doc = "Bit 2 - Enable receiver line status interrupt"]
    #[inline(always)]
    pub fn elsi(&self) -> ElsiR {
        ElsiR::new(((self.bits >> 2) & 1) != 0)
    }
    #[doc = "Bit 3 - Enable modem status interrupt"]
    #[inline(always)]
    pub fn edssi(&self) -> EdssiR {
        EdssiR::new(((self.bits >> 3) & 1) != 0)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("IER")
            .field("erbfi", &self.erbfi())
            .field("etbei", &self.etbei())
            .field("elsi", &self.elsi())
            .field("edssi", &self.edssi())
            .finish()
    }
}
impl W {
    #[doc = "Bit 0 - Enable received data available interrupt"]
    #[inline(always)]
    pub fn erbfi(&mut self) -> ErbfiW<'_, IerSpec> {
        ErbfiW::new(self, 0)
    }
    #[doc = "Bit 1 - Enable transmitter holding register empty interrupt"]
    #[inline(always)]
    pub fn etbei(&mut self) -> EtbeiW<'_, IerSpec> {
        EtbeiW::new(self, 1)
    }
    #[doc = "Bit 2 - Enable receiver line status interrupt"]
    #[inline(always)]
    pub fn elsi(&mut self) -> ElsiW<'_, IerSpec> {
        ElsiW::new(self, 2)
    }
    #[doc = "Bit 3 - Enable modem status interrupt"]
    #[inline(always)]
    pub fn edssi(&mut self) -> EdssiW<'_, IerSpec> {
        EdssiW::new(self, 3)
    }
}
#[doc = "Interrupt Enable Register\n\nYou can [`read`](crate::Reg::read) this register and get [`ier::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`ier::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct IerSpec;
impl crate::RegisterSpec for IerSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`ier::R`](R) reader structure"]
impl crate::Readable for IerSpec {}
#[doc = "`write(|w| ..)` method takes [`ier::W`](W) writer structure"]
impl crate::Writable for IerSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets IER to value 0"]
impl crate::Resettable for IerSpec {}
