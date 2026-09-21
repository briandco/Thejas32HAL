#[doc = "Register `DIR` reader"]
pub type R = crate::R<DirSpec>;
#[doc = "Register `DIR` writer"]
pub type W = crate::W<DirSpec>;
#[doc = "Field `PIN` reader - Direction bits for pins 0-15"]
pub type PinR = crate::FieldReader<u16>;
#[doc = "Field `PIN` writer - Direction bits for pins 0-15"]
pub type PinW<'a, REG> = crate::FieldWriter<'a, REG, 16, u16>;
impl R {
    #[doc = "Bits 0:15 - Direction bits for pins 0-15"]
    #[inline(always)]
    pub fn pin(&self) -> PinR {
        PinR::new(self.bits)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("DIR").field("pin", &self.pin()).finish()
    }
}
impl W {
    #[doc = "Bits 0:15 - Direction bits for pins 0-15"]
    #[inline(always)]
    pub fn pin(&mut self) -> PinW<'_, DirSpec> {
        PinW::new(self, 0)
    }
}
#[doc = "GPIO direction register (0=input, 1=output)\n\nYou can [`read`](crate::Reg::read) this register and get [`dir::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`dir::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct DirSpec;
impl crate::RegisterSpec for DirSpec {
    type Ux = u16;
}
#[doc = "`read()` method returns [`dir::R`](R) reader structure"]
impl crate::Readable for DirSpec {}
#[doc = "`write(|w| ..)` method takes [`dir::W`](W) writer structure"]
impl crate::Writable for DirSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets DIR to value 0"]
impl crate::Resettable for DirSpec {}
