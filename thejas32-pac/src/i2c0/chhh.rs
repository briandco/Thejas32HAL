#[doc = "Register `CHHH` reader"]
pub type R = crate::R<ChhhSpec>;
#[doc = "Register `CHHH` writer"]
pub type W = crate::W<ChhhSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
impl W {}
#[doc = "Clock High Period (extended)\n\nYou can [`read`](crate::Reg::read) this register and get [`chhh::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`chhh::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct ChhhSpec;
impl crate::RegisterSpec for ChhhSpec {
    type Ux = u8;
}
#[doc = "`read()` method returns [`chhh::R`](R) reader structure"]
impl crate::Readable for ChhhSpec {}
#[doc = "`write(|w| ..)` method takes [`chhh::W`](W) writer structure"]
impl crate::Writable for ChhhSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets CHHH to value 0"]
impl crate::Resettable for ChhhSpec {}
