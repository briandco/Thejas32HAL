#[doc = "Register `DATAOUT` reader"]
pub type R = crate::R<DataoutSpec>;
#[doc = "Register `DATAOUT` writer"]
pub type W = crate::W<DataoutSpec>;
#[doc = "Field `PINS` reader - All pin values"]
pub type PinsR = crate::FieldReader<u16>;
#[doc = "Field `PINS` writer - All pin values"]
pub type PinsW<'a, REG> = crate::FieldWriter<'a, REG, 16, u16>;
impl R {
    #[doc = "Bits 0:15 - All pin values"]
    #[inline(always)]
    pub fn pins(&self) -> PinsR {
        PinsR::new(self.bits)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("DATAOUT")
            .field("pins", &self.pins())
            .finish()
    }
}
impl W {
    #[doc = "Bits 0:15 - All pin values"]
    #[inline(always)]
    pub fn pins(&mut self) -> PinsW<'_, DataoutSpec> {
        PinsW::new(self, 0)
    }
}
#[doc = "GPIO data register (all 16 pins, bulk read/write)\n\nYou can [`read`](crate::Reg::read) this register and get [`dataout::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`dataout::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct DataoutSpec;
impl crate::RegisterSpec for DataoutSpec {
    type Ux = u16;
}
#[doc = "`read()` method returns [`dataout::R`](R) reader structure"]
impl crate::Readable for DataoutSpec {}
#[doc = "`write(|w| ..)` method takes [`dataout::W`](W) writer structure"]
impl crate::Writable for DataoutSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets DATAOUT to value 0"]
impl crate::Resettable for DataoutSpec {}
