#[doc = "Register `CHH` reader"]
pub type R = crate::R<ChhSpec>;
#[doc = "Register `CHH` writer"]
pub type W = crate::W<ChhSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
impl W {}
#[doc = "Clock High Period\n\nYou can [`read`](crate::Reg::read) this register and get [`chh::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`chh::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct ChhSpec;
impl crate::RegisterSpec for ChhSpec {
    type Ux = u8;
}
#[doc = "`read()` method returns [`chh::R`](R) reader structure"]
impl crate::Readable for ChhSpec {}
#[doc = "`write(|w| ..)` method takes [`chh::W`](W) writer structure"]
impl crate::Writable for ChhSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets CHH to value 0"]
impl crate::Resettable for ChhSpec {}
