#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinzLocationId {
    /// Kaitia
    KTIA,
    /// Whangarei
    WHNG,
    /// Warkworth
    WARK,
    /// Auckland
    AUCK,
    /// Coromandel
    CORM,
    /// Hamilton
    HAMT,
    /// Taranga
    TRNG,
    /// Hikurangi
    HIKB,
    /// Mahurangi
    MAHO,
    /// Taupo
    TAUP,
    /// Whakatane
    WHTK,
    /// Gisborne
    GISB,
    /// Napier
    NPLY,
    /// VGM
    VGMT,
    /// Wanganui
    WANG,
    /// Hastings
    HAST,
    /// Dannevirke
    DNVK,
    /// Wairarapa
    WRPA,
    /// Wellington
    WGTN,
    /// Golden Bay
    GLDB,
    /// Nelson
    NLSN,
    /// Westport
    WEST,
    /// Kaikoura
    KAIK,
    /// Murchison
    MQZG,
    /// Hokitika
    HOKI,
    /// Lake Taupo
    LKTA,
    /// Methven
    METH,
    /// Hastings
    HAAS,
    /// Mount John
    MTJO,
    /// Waimakariri
    WAIM,
    /// Mavora
    MAVL,
    /// Alexandra
    LEXA,
    /// Dunedin
    DUND,
    /// Invercargill
    INVR,
    /// Bluff
    BLUF,
    /// ??
    PYGR,
    /// Chatham Islands
    CHTI,
}

impl AsRef<str> for LinzLocationId {
    fn as_ref(&self) -> &str {
        match self {
            LinzLocationId::KTIA => "KTIA",
            LinzLocationId::WHNG => "WHNG",
            LinzLocationId::WARK => "WARK",
            LinzLocationId::AUCK => "AUCK",
            LinzLocationId::CORM => "CORM",
            LinzLocationId::HAMT => "HAMT",
            LinzLocationId::TRNG => "TRNG",
            LinzLocationId::HIKB => "HIKB",
            LinzLocationId::MAHO => "MAHO",
            LinzLocationId::TAUP => "TAUP",
            LinzLocationId::WHTK => "WHTK",
            LinzLocationId::GISB => "GISB",
            LinzLocationId::NPLY => "NPLY",
            LinzLocationId::VGMT => "VGMT",
            LinzLocationId::WANG => "WANG",
            LinzLocationId::HAST => "HAST",
            LinzLocationId::DNVK => "DNVK",
            LinzLocationId::WRPA => "WRPA",
            LinzLocationId::WGTN => "WGTN",
            LinzLocationId::GLDB => "GLDB",
            LinzLocationId::NLSN => "NLSN",
            LinzLocationId::WEST => "WEST",
            LinzLocationId::KAIK => "KAIK",
            LinzLocationId::MQZG => "MQZG",
            LinzLocationId::HOKI => "HOKI",
            LinzLocationId::LKTA => "LKTA",
            LinzLocationId::METH => "METH",
            LinzLocationId::HAAS => "HAAS",
            LinzLocationId::MTJO => "MTJO",
            LinzLocationId::WAIM => "WAIM",
            LinzLocationId::MAVL => "MAVL",
            LinzLocationId::LEXA => "LEXA",
            LinzLocationId::DUND => "DUND",
            LinzLocationId::INVR => "INVR",
            LinzLocationId::BLUF => "BLUF",
            LinzLocationId::PYGR => "PYGR",
            LinzLocationId::CHTI => "CHTI",
        }
    }
}

impl LinzLocationId {
    /// Fetch the lat / long location for a Linz location
    /// (this seems to need to be manually copied from
    /// https://www.geodesy.linz.govt.nz/positionzrt/)
    pub fn location(&self) -> (f32, f32) {
        match self {
            Self::AUCK => (-36.60, 174.83),
            _ => unimplemented!(),
        }
    }
}
