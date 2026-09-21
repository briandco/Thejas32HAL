#[doc = "Register `SW_INTR_EN` reader"]
pub type R = crate::R<SwIntrEnSpec>;
#[doc = "Register `SW_INTR_EN` writer"]
pub type W = crate::W<SwIntrEnSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
impl W {}
#[doc = "Software interrupt enable\n\nYou can [`read`](crate::Reg::read) this register and get [`sw_intr_en::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`sw_intr_en::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct SwIntrEnSpec;
impl crate::RegisterSpec for SwIntrEnSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`sw_intr_en::R`](R) reader structure"]
impl crate::Readable for SwIntrEnSpec {}
#[doc = "`write(|w| ..)` method takes [`sw_intr_en::W`](W) writer structure"]
impl crate::Writable for SwIntrEnSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets SW_INTR_EN to value 0"]
impl crate::Resettable for SwIntrEnSpec {}
