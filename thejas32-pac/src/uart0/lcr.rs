#[doc = "Register `LCR` reader"]
pub type R = crate::R<LcrSpec>;
#[doc = "Register `LCR` writer"]
pub type W = crate::W<LcrSpec>;
#[doc = "Field `WLS` reader - Word length select (0=5bit,1=6bit,2=7bit,3=8bit)"]
pub type WlsR = crate::FieldReader;
#[doc = "Field `WLS` writer - Word length select (0=5bit,1=6bit,2=7bit,3=8bit)"]
pub type WlsW<'a, REG> = crate::FieldWriter<'a, REG, 2>;
#[doc = "Field `STB` reader - Number of stop bits"]
pub type StbR = crate::BitReader;
#[doc = "Field `STB` writer - Number of stop bits"]
pub type StbW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `PEN` reader - Parity enable"]
pub type PenR = crate::BitReader;
#[doc = "Field `PEN` writer - Parity enable"]
pub type PenW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `EPS` reader - Even parity select"]
pub type EpsR = crate::BitReader;
#[doc = "Field `EPS` writer - Even parity select"]
pub type EpsW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `STKP` reader - Stick parity"]
pub type StkpR = crate::BitReader;
#[doc = "Field `STKP` writer - Stick parity"]
pub type StkpW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `SBRK` reader - Set break"]
pub type SbrkR = crate::BitReader;
#[doc = "Field `SBRK` writer - Set break"]
pub type SbrkW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `DLAB` reader - Divisor latch access bit"]
pub type DlabR = crate::BitReader;
#[doc = "Field `DLAB` writer - Divisor latch access bit"]
pub type DlabW<'a, REG> = crate::BitWriter<'a, REG>;
impl R {
    #[doc = "Bits 0:1 - Word length select (0=5bit,1=6bit,2=7bit,3=8bit)"]
    #[inline(always)]
    pub fn wls(&self) -> WlsR {
        WlsR::new((self.bits & 3) as u8)
    }
    #[doc = "Bit 2 - Number of stop bits"]
    #[inline(always)]
    pub fn stb(&self) -> StbR {
        StbR::new(((self.bits >> 2) & 1) != 0)
    }
    #[doc = "Bit 3 - Parity enable"]
    #[inline(always)]
    pub fn pen(&self) -> PenR {
        PenR::new(((self.bits >> 3) & 1) != 0)
    }
    #[doc = "Bit 4 - Even parity select"]
    #[inline(always)]
    pub fn eps(&self) -> EpsR {
        EpsR::new(((self.bits >> 4) & 1) != 0)
    }
    #[doc = "Bit 5 - Stick parity"]
    #[inline(always)]
    pub fn stkp(&self) -> StkpR {
        StkpR::new(((self.bits >> 5) & 1) != 0)
    }
    #[doc = "Bit 6 - Set break"]
    #[inline(always)]
    pub fn sbrk(&self) -> SbrkR {
        SbrkR::new(((self.bits >> 6) & 1) != 0)
    }
    #[doc = "Bit 7 - Divisor latch access bit"]
    #[inline(always)]
    pub fn dlab(&self) -> DlabR {
        DlabR::new(((self.bits >> 7) & 1) != 0)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("LCR")
            .field("wls", &self.wls())
            .field("stb", &self.stb())
            .field("pen", &self.pen())
            .field("eps", &self.eps())
            .field("stkp", &self.stkp())
            .field("sbrk", &self.sbrk())
            .field("dlab", &self.dlab())
            .finish()
    }
}
impl W {
    #[doc = "Bits 0:1 - Word length select (0=5bit,1=6bit,2=7bit,3=8bit)"]
    #[inline(always)]
    pub fn wls(&mut self) -> WlsW<'_, LcrSpec> {
        WlsW::new(self, 0)
    }
    #[doc = "Bit 2 - Number of stop bits"]
    #[inline(always)]
    pub fn stb(&mut self) -> StbW<'_, LcrSpec> {
        StbW::new(self, 2)
    }
    #[doc = "Bit 3 - Parity enable"]
    #[inline(always)]
    pub fn pen(&mut self) -> PenW<'_, LcrSpec> {
        PenW::new(self, 3)
    }
    #[doc = "Bit 4 - Even parity select"]
    #[inline(always)]
    pub fn eps(&mut self) -> EpsW<'_, LcrSpec> {
        EpsW::new(self, 4)
    }
    #[doc = "Bit 5 - Stick parity"]
    #[inline(always)]
    pub fn stkp(&mut self) -> StkpW<'_, LcrSpec> {
        StkpW::new(self, 5)
    }
    #[doc = "Bit 6 - Set break"]
    #[inline(always)]
    pub fn sbrk(&mut self) -> SbrkW<'_, LcrSpec> {
        SbrkW::new(self, 6)
    }
    #[doc = "Bit 7 - Divisor latch access bit"]
    #[inline(always)]
    pub fn dlab(&mut self) -> DlabW<'_, LcrSpec> {
        DlabW::new(self, 7)
    }
}
#[doc = "Line Control Register\n\nYou can [`read`](crate::Reg::read) this register and get [`lcr::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`lcr::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct LcrSpec;
impl crate::RegisterSpec for LcrSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`lcr::R`](R) reader structure"]
impl crate::Readable for LcrSpec {}
#[doc = "`write(|w| ..)` method takes [`lcr::W`](W) writer structure"]
impl crate::Writable for LcrSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets LCR to value 0"]
impl crate::Resettable for LcrSpec {}
