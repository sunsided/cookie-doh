//! Hand-drawn sprites. Color key: [`crate::gfx::palette::from_char`].

use crate::gfx::Sprite;

pub struct Art {
    pub cookie: Sprite,
    pub golden: Sprite,
    /// 4x4 cookie for the jar pile.
    pub mini: Sprite,
    pub mini_golden: Sprite,
    /// 5x5 cookie for the HUD.
    pub life: Sprite,
    pub life_lost: Sprite,
    pub body: Sprite,
    pub mouth_happy: Sprite,
    pub mouth_doh: Sprite,
    pub mouth_worry: Sprite,
    pub brows_doh: Sprite,
    pub hand: Sprite,
    pub jar_back: Sprite,
    pub jar_front: Sprite,
    pub spider: [Sprite; 2],
    pub lock: Sprite,
    pub moon: Sprite,
}

const GOLD: &[(char, char)] = &[('d', 'u'), ('c', 'o'), ('C', 'y'), ('h', 'Y'), ('x', 'o')];

impl Art {
    pub fn new() -> Self {
        Self {
            cookie: Sprite::parse(COOKIE),
            golden: Sprite::parse_with(COOKIE, GOLD),
            mini: Sprite::parse(MINI),
            mini_golden: Sprite::parse_with(MINI, GOLD),
            life: Sprite::parse(LIFE),
            life_lost: Sprite::parse_with(
                LIFE,
                &[('c', 'z'), ('C', 'a'), ('h', 'A'), ('x', 'z'), ('d', 'k')],
            ),
            body: Sprite::parse(BODY),
            mouth_happy: Sprite::parse(MOUTH_HAPPY),
            mouth_doh: Sprite::parse(MOUTH_DOH),
            mouth_worry: Sprite::parse(MOUTH_WORRY),
            brows_doh: Sprite::parse(BROWS_DOH),
            hand: Sprite::parse(HAND),
            jar_back: Sprite::parse(JAR_BACK),
            jar_front: Sprite::parse(JAR_FRONT),
            spider: [Sprite::parse(SPIDER_A), Sprite::parse(SPIDER_B)],
            lock: Sprite::parse(LOCK),
            moon: Sprite::parse(MOON),
        }
    }
}

impl Default for Art {
    fn default() -> Self {
        Self::new()
    }
}

const COOKIE: &str = "
    ..dccd..
    .cChCCc.
    dCxCChCd
    cCCCxCCc
    cChCCCxc
    dCCxCCCd
    .cCCChc.
    ..dccd..
";

const MINI: &str = "
    .cC.
    cxCc
    CChx
    .cc.
";

const LIFE: &str = "
    .dcd.
    cCxCc
    dhCCd
    cCCxc
    .dcd.
";

/// The protagonist, front view. The jar covers rows 16..30, columns 4..16.
const BODY: &str = "
    .......dddddd.......
    .....dduuuuuudd.....
    ....duuuuuuuuuud....
    ...duuduuuuuduuud...
    ..duuuuuuuuuuuuuud..
    ..dufffuuffuufffud..
    ..dffffffffffffffd..
    ..efkkkkkffkkkkkfe..
    ..efkwkikkkkwkikfe..
    ..efkkkkkffkkkkkfe..
    ..effffffeeffffffe..
    ..effffffffffffffe..
    ...effffkkkkffffe...
    ...eeffffffffffee...
    ....eeeffffffeee....
    .......effffe.......
    ....gGGGGGGGGGGg....
    ..gGGGGGGGGGGGGGGg..
    .gGGGGGGcCCcGGGGGGg.
    .gGGGGGGCxCCGGGGGGg.
    .gGGGGGGcCCcGGGGGGg.
    .gGGGGGGGGGGGGGGGGg.
    .gGGGgGGGGGGGGgGGGg.
    .gGGGggggggggggGGGg.
    ..gggggggggggggggg..
    ...jBBBBBjjBBBBBj...
    ...jBBBBBjjBBBBBj...
    ...jBBBBj..jBBBBj...
    ...jBBBBj..jBBBBj...
    ...jjjjjj..jjjjjj...
    ..kzzzzzk..kzzzzzk..
    .kkkkkkkk..kkkkkkkk.
";

/// Drawn at body (7, 12).
const MOUTH_HAPPY: &str = "
    kffffk
    fkRRkf
    ffkkff
";

/// Drawn at body (7, 11).
const MOUTH_DOH: &str = "
    fkkkkf
    kRRRRk
    kRPPRk
    fkkkkf
";

/// Drawn at body (7, 12).
const MOUTH_WORRY: &str = "
    ffkkff
    fkffkf
    ffffff
";

/// Raised brows plus squeezed-shut eyes, drawn at body (3, 5).
const BROWS_DOH: &str = "
    .dd.......dd.
    d..d.....d..d
    .............
    .kkkkk.kkkkk.
    .kikik.kikik.
    .kkkkk.kkkkk.
";

const HAND: &str = "
    eff
    fFf
    eff
";

const JAR_BACK: &str = "
    ............
    ...tttttt...
    ...tttttt...
    ..tttttttt..
    .tttttttttt.
    .tttttttttt.
    .tttttttttt.
    .tttttttttt.
    .tttttttttt.
    .tttttttttt.
    .tttttttttt.
    .tttttttttt.
    ..tttttttt..
    ............
";

const JAR_FRONT: &str = "
    .AAAAAAAAAA.
    .Al......lA.
    ..l......l..
    .lw.......l.
    lw........pl
    lw.........l
    lw.........l
    lw.........l
    lw.........l
    l..........l
    l..........l
    l.........pl
    .l........l.
    ..llllllll..
";

const SPIDER_A: &str = "
    .k...........k.
    ..k..vvvvv..k..
    k..kvVVVVVvk..k
    .k.kvVwVVVvk.k.
    ..kkvVVVVVvkk..
    kkk.vvvvvvv.kkk
    ...kvwRvwRvk...
    ..k.kvwvwvk.k..
    .k..k.kkk.k..k.
    k..k.......k..k
";

const SPIDER_B: &str = "
    k.............k
    .k...vvvvv...k.
    ..kkvVVVVVvkk..
    k..kvVwVVVvk..k
    .kkkvVVVVVvkkk.
    ....vvvvvvv....
    ..kkvwRvwRvkk..
    .k..kvwvwvk..k.
    k..k.......k..k
    ...k.......k...
";

const LOCK: &str = "
    .aa.
    a..a
    yyyy
    yoyy
    yyyy
";

const MOON: &str = "
    ...pppp...
    ..pwwwwpp.
    .pwwwpwwpp
    pwwwwwwwpl
    pwpwwwwwpl
    pwwwwwpwpl
    pwwwwwwwpl
    .pwwpwwppl
    ..ppwwppl.
    ...lllll..
";

#[cfg(test)]
mod tests {
    #[test]
    fn all_sprites_parse() {
        let a = super::Art::new();
        assert_eq!((a.body.w, a.body.h), (20, 32));
        assert_eq!((a.spider[0].w, a.spider[1].h), (15, 10));
        assert_eq!(a.jar_back.w, a.jar_front.w);
        assert_eq!(a.jar_back.h, a.jar_front.h);
    }
}
