#[doc = "Register `CHHL` reader"]
pub type R = crate::R<ChhlSpec>;
#[doc = "Register `CHHL` writer"]
pub type W = crate::W<ChhlSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
impl W {}
#[doc = "Clock Low Period (extended)\n\nYou can [`read`](crate::Reg::read) this register and get [`chhl::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`chhl::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct ChhlSpec;
impl crate::RegisterSpec for ChhlSpec {
    type Ux = u8;
}
#[doc = "`read()` method returns [`chhl::R`](R) reader structure"]
impl crate::Readable for ChhlSpec {}
#[doc = "`write(|w| ..)` method takes [`chhl::W`](W) writer structure"]
impl crate::Writable for ChhlSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets CHHL to value 0x3e"]
impl crate::Resettable for ChhlSpec {
    const RESET_VALUE: u8 = 0x3e;
}
