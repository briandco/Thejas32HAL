#[doc = "Register `LOAD_COUNT` reader"]
pub type R = crate::R<LoadCountSpec>;
#[doc = "Register `LOAD_COUNT` writer"]
pub type W = crate::W<LoadCountSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
impl W {}
#[doc = "Timer load count value\n\nYou can [`read`](crate::Reg::read) this register and get [`load_count::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`load_count::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct LoadCountSpec;
impl crate::RegisterSpec for LoadCountSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`load_count::R`](R) reader structure"]
impl crate::Readable for LoadCountSpec {}
#[doc = "`write(|w| ..)` method takes [`load_count::W`](W) writer structure"]
impl crate::Writable for LoadCountSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets LOAD_COUNT to value 0"]
impl crate::Resettable for LoadCountSpec {}
