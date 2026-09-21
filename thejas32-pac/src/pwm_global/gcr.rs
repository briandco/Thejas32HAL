#[doc = "Register `GCR` reader"]
pub type R = crate::R<GcrSpec>;
#[doc = "Register `GCR` writer"]
pub type W = crate::W<GcrSpec>;
#[doc = "Field `GPE` reader - Global PWM enable"]
pub type GpeR = crate::BitReader;
#[doc = "Field `GPE` writer - Global PWM enable"]
pub type GpeW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `GIE` reader - Global interrupt enable"]
pub type GieR = crate::BitReader;
#[doc = "Field `GIE` writer - Global interrupt enable"]
pub type GieW<'a, REG> = crate::BitWriter<'a, REG>;
impl R {
    #[doc = "Bit 0 - Global PWM enable"]
    #[inline(always)]
    pub fn gpe(&self) -> GpeR {
        GpeR::new((self.bits & 1) != 0)
    }
    #[doc = "Bit 1 - Global interrupt enable"]
    #[inline(always)]
    pub fn gie(&self) -> GieR {
        GieR::new(((self.bits >> 1) & 1) != 0)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("GCR")
            .field("gpe", &self.gpe())
            .field("gie", &self.gie())
            .finish()
    }
}
impl W {
    #[doc = "Bit 0 - Global PWM enable"]
    #[inline(always)]
    pub fn gpe(&mut self) -> GpeW<'_, GcrSpec> {
        GpeW::new(self, 0)
    }
    #[doc = "Bit 1 - Global interrupt enable"]
    #[inline(always)]
    pub fn gie(&mut self) -> GieW<'_, GcrSpec> {
        GieW::new(self, 1)
    }
}
#[doc = "PWM Global Control Register\n\nYou can [`read`](crate::Reg::read) this register and get [`gcr::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`gcr::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct GcrSpec;
impl crate::RegisterSpec for GcrSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`gcr::R`](R) reader structure"]
impl crate::Readable for GcrSpec {}
#[doc = "`write(|w| ..)` method takes [`gcr::W`](W) writer structure"]
impl crate::Writable for GcrSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets GCR to value 0"]
impl crate::Resettable for GcrSpec {}
