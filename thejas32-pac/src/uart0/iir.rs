#[doc = "Register `IIR` reader"]
pub type R = crate::R<IirSpec>;
#[doc = "Field `IID` reader - Interrupt identification"]
pub type IidR = crate::FieldReader;
#[doc = "Field `FIFOEN` reader - FIFO enable status"]
pub type FifoenR = crate::FieldReader;
impl R {
    #[doc = "Bits 0:3 - Interrupt identification"]
    #[inline(always)]
    pub fn iid(&self) -> IidR {
        IidR::new((self.bits & 0x0f) as u8)
    }
    #[doc = "Bits 6:7 - FIFO enable status"]
    #[inline(always)]
    pub fn fifoen(&self) -> FifoenR {
        FifoenR::new(((self.bits >> 6) & 3) as u8)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("IIR")
            .field("iid", &self.iid())
            .field("fifoen", &self.fifoen())
            .finish()
    }
}
#[doc = "Interrupt Identification Register\n\nYou can [`read`](crate::Reg::read) this register and get [`iir::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct IirSpec;
impl crate::RegisterSpec for IirSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`iir::R`](R) reader structure"]
impl crate::Readable for IirSpec {}
#[doc = "`reset()` method sets IIR to value 0x01"]
impl crate::Resettable for IirSpec {
    const RESET_VALUE: u32 = 0x01;
}
