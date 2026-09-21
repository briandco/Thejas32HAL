#[doc = "Register `CHL` reader"]
pub type R = crate::R<ChlSpec>;
#[doc = "Register `CHL` writer"]
pub type W = crate::W<ChlSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
impl W {}
#[doc = "Clock Low Period\n\nYou can [`read`](crate::Reg::read) this register and get [`chl::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`chl::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct ChlSpec;
impl crate::RegisterSpec for ChlSpec {
    type Ux = u8;
}
#[doc = "`read()` method returns [`chl::R`](R) reader structure"]
impl crate::Readable for ChlSpec {}
#[doc = "`write(|w| ..)` method takes [`chl::W`](W) writer structure"]
impl crate::Writable for ChlSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets CHL to value 0x7d"]
impl crate::Resettable for ChlSpec {
    const RESET_VALUE: u8 = 0x7d;
}
