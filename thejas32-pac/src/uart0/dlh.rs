#[doc = "Register `DLH` reader"]
pub type R = crate::R<DlhSpec>;
#[doc = "Register `DLH` writer"]
pub type W = crate::W<DlhSpec>;
#[doc = "Field `DATA` reader - Divisor latch high byte when LCR.DLAB=1"]
pub type DataR = crate::FieldReader;
#[doc = "Field `DATA` writer - Divisor latch high byte when LCR.DLAB=1"]
pub type DataW<'a, REG> = crate::FieldWriter<'a, REG, 8>;
impl R {
    #[doc = "Bits 0:7 - Divisor latch high byte when LCR.DLAB=1"]
    #[inline(always)]
    pub fn data(&self) -> DataR {
        DataR::new((self.bits & 0xff) as u8)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("DLH").field("data", &self.data()).finish()
    }
}
impl W {
    #[doc = "Bits 0:7 - Divisor latch high byte when LCR.DLAB=1"]
    #[inline(always)]
    pub fn data(&mut self) -> DataW<'_, DlhSpec> {
        DataW::new(self, 0)
    }
}
#[doc = "Divisor Latch High Register\n\nYou can [`read`](crate::Reg::read) this register and get [`dlh::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`dlh::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct DlhSpec;
impl crate::RegisterSpec for DlhSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`dlh::R`](R) reader structure"]
impl crate::Readable for DlhSpec {}
#[doc = "`write(|w| ..)` method takes [`dlh::W`](W) writer structure"]
impl crate::Writable for DlhSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets DLH to value 0"]
impl crate::Resettable for DlhSpec {}
