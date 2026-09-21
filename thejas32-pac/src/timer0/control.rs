#[doc = "Register `CONTROL` reader"]
pub type R = crate::R<ControlSpec>;
#[doc = "Register `CONTROL` writer"]
pub type W = crate::W<ControlSpec>;
#[doc = "Field `ENABLE` reader - Timer enable"]
pub type EnableR = crate::BitReader;
#[doc = "Field `ENABLE` writer - Timer enable"]
pub type EnableW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `MODE` reader - Timer mode (0=free-running, 1=user-defined)"]
pub type ModeR = crate::BitReader;
#[doc = "Field `MODE` writer - Timer mode (0=free-running, 1=user-defined)"]
pub type ModeW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `INTR_MASK` reader - Interrupt mask (0=unmasked, 1=masked)"]
pub type IntrMaskR = crate::BitReader;
#[doc = "Field `INTR_MASK` writer - Interrupt mask (0=unmasked, 1=masked)"]
pub type IntrMaskW<'a, REG> = crate::BitWriter<'a, REG>;
impl R {
    #[doc = "Bit 0 - Timer enable"]
    #[inline(always)]
    pub fn enable(&self) -> EnableR {
        EnableR::new((self.bits & 1) != 0)
    }
    #[doc = "Bit 1 - Timer mode (0=free-running, 1=user-defined)"]
    #[inline(always)]
    pub fn mode(&self) -> ModeR {
        ModeR::new(((self.bits >> 1) & 1) != 0)
    }
    #[doc = "Bit 2 - Interrupt mask (0=unmasked, 1=masked)"]
    #[inline(always)]
    pub fn intr_mask(&self) -> IntrMaskR {
        IntrMaskR::new(((self.bits >> 2) & 1) != 0)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("CONTROL")
            .field("enable", &self.enable())
            .field("mode", &self.mode())
            .field("intr_mask", &self.intr_mask())
            .finish()
    }
}
impl W {
    #[doc = "Bit 0 - Timer enable"]
    #[inline(always)]
    pub fn enable(&mut self) -> EnableW<'_, ControlSpec> {
        EnableW::new(self, 0)
    }
    #[doc = "Bit 1 - Timer mode (0=free-running, 1=user-defined)"]
    #[inline(always)]
    pub fn mode(&mut self) -> ModeW<'_, ControlSpec> {
        ModeW::new(self, 1)
    }
    #[doc = "Bit 2 - Interrupt mask (0=unmasked, 1=masked)"]
    #[inline(always)]
    pub fn intr_mask(&mut self) -> IntrMaskW<'_, ControlSpec> {
        IntrMaskW::new(self, 2)
    }
}
#[doc = "Timer control register\n\nYou can [`read`](crate::Reg::read) this register and get [`control::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`control::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
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
