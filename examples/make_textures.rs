//! Development tool, NOT part of the renderer.
//!
//! Turns the hand-drawn 16×16 pixel art below (one letter = one pixel, each
//! texture has its own small palette) into BMP files in `assets/textures/`.
//! The renderer only ever reads those BMP files.
//!
//!     cargo run --example make_textures                  (only writes missing files)
//!     cargo run --example make_textures -- --overwrite   (rewrites all of them)
//!
//! Existing BMPs are kept by default, so textures you repainted in MS Paint
//! are safe. `--overwrite` resets every texture to the drawings below.

use std::fs;
use std::io::Write;
use std::path::Path;

struct Art {
    name: &'static str,
    palette: &'static [(char, [u8; 3])],
    rows: [&'static str; 16],
}

const GRASS: &[(char, [u8; 3])] = &[
    ('g', [106, 170, 74]),
    ('G', [88, 150, 60]),
    ('l', [128, 192, 90]),
    ('d', [72, 128, 52]),
];
const DIRT: &[(char, [u8; 3])] = &[
    ('b', [134, 96, 67]),
    ('B', [112, 78, 54]),
    ('c', [158, 116, 84]),
    ('s', [94, 64, 44]),
];
const GRASS_SIDE: &[(char, [u8; 3])] = &[
    ('g', [106, 170, 74]),
    ('G', [88, 150, 60]),
    ('l', [128, 192, 90]),
    ('b', [134, 96, 67]),
    ('B', [112, 78, 54]),
    ('c', [158, 116, 84]),
    ('s', [94, 64, 44]),
];

const TEXTURES: &[Art] = &[
    Art {
        name: "grass_top",
        palette: GRASS,
        rows: [
            "gGglgggGgglggGgg",
            "ggGgggdgGgggglgG",
            "lgggGggggglgGggg",
            "gGgdgglgGgggggdg",
            "ggglgGggggGglggg",
            "GgggggglgdggggGg",
            "gglgGgGggggGgglg",
            "gdggggggglgggGgg",
            "gggGglggGgggdggg",
            "lgGgggdgggglgggG",
            "ggggGggglGgggGgg",
            "gGlggggGgggggdgl",
            "ggggdglggGglgggg",
            "gGgggggGgggggGgg",
            "lggGglgggdgGgglg",
            "ggdgggGglggggggG",
        ],
    },
    Art {
        name: "grass_side",
        palette: GRASS_SIDE,
        rows: [
            "gGglgggGgglggGgg",
            "gggGgggggGgggggl",
            "GgbgGglgbggGgbgg",
            "bgbbgbgGbbgbbbgb",
            "bbcbbbbgbbcbbbbb",
            "cbbBbbbbbbbbBbcb",
            "bbbbbcbbBbbbbbbb",
            "bBbbbbbbbbcbbsbb",
            "bbbsbbcbbbbbbbbc",
            "cbbbbbbbBbbbcbbb",
            "bbBbbcbbbbbsbbbb",
            "bbbbbbbbcbbbbBbb",
            "bsbbcbBbbbbbbbbb",
            "bbbbbbbbbsbbcbbb",
            "bcbBbbbbbbbbbbbs",
            "bbbbbsbbcbBbbbbb",
        ],
    },
    Art {
        name: "dirt",
        palette: DIRT,
        rows: [
            "bbcbbbbsbbcbbbbb",
            "cbbBbbbbbbbbBbcb",
            "bbbbbcbbBbbbbbbb",
            "bBbbbbbbbbcbbsbb",
            "bbbsbbcbbbbbbbbc",
            "cbbbbbbbBbbbcbbb",
            "bbBbbcbbbbbsbbbb",
            "bbbbbbbbcbbbbBbb",
            "bsbbcbBbbbbbbbbb",
            "bbbbbbbbbsbbcbbb",
            "bcbBbbbbbbbbbbbs",
            "bbbbbsbbcbBbbbbb",
            "Bbbcbbbbbbbbbcbb",
            "bbbbbBbbsbbbbbbb",
            "bcbbbbbbbbbcbbBb",
            "bbbsbbcbbBbbbbbb",
        ],
    },
    Art {
        name: "rock",
        palette: &[
            ('r', [125, 125, 130]),
            ('R', [105, 105, 112]),
            ('h', [150, 150, 156]),
            ('k', [84, 84, 92]),
        ],
        rows: [
            "rrrhrrrRrrrrrhrr",
            "rRrrrrkrrrhrrrrR",
            "rrrrRrkrrrrrRrrr",
            "hrrrrrrkkrrrrrrh",
            "rrRrrrrrrkrrRrrr",
            "rrrrhrrRrrrrrrrr",
            "Rrrrrrrrrrrhrrkr",
            "rrkkrrrrRrrrrkrr",
            "rrrrkkrrrrrrkrrR",
            "rhrrrrkrrRrrrrrr",
            "rrrRrrrrrrrhrrrr",
            "rrrrrrrhrrrrrRrr",
            "kRrrrrrrrrkkrrrr",
            "rrkrrRrrrkrrrrhr",
            "rrrkrrrrrrrrRrrr",
            "hrrrrrhrrRrrrrrr",
        ],
    },
    Art {
        // Flagstones with mortar lines; the lower row of stones is offset
        // by half a stone, like a real paved path.
        name: "path_stone",
        palette: &[
            ('p', [202, 194, 180]),
            ('P', [182, 174, 162]),
            ('w', [218, 211, 199]),
            ('m', [138, 130, 120]),
        ],
        rows: [
            "mmmmmmmmmmmmmmmm",
            "mpppwpppmPPPPwPP",
            "mppppppPmPPPPPPP",
            "mpPpppppmPPwPPPP",
            "mppppwppmPPPPPPP",
            "mpppppppmPwPPPPP",
            "mpppPpppmPPPPPPw",
            "mpppppppmPPPPPPP",
            "mmmmmmmmmmmmmmmm",
            "PPPPmpppwpppmPPP",
            "PPwPmpppppppmPPw",
            "PPPPmppPppppmPPP",
            "PPPPmpppppwpmPPP",
            "wPPPmpppppppmPPP",
            "PPPPmpwpppppmPPP",
            "PPPPmpppppPpmPPP",
        ],
    },
    Art {
        // Smooth vermilion lacquer with a faint vertical grain and one
        // bright streak where the gloss catches the light.
        name: "lacquer",
        palette: &[
            ('v', [206, 56, 38]),
            ('V', [182, 44, 30]),
            ('h', [232, 92, 64]),
        ],
        rows: [
            "vhvvVvvvhvvvVvvv",
            "vhvvVvvvhvvvVvvv",
            "vhvvvvvvhvvvVvvv",
            "vhvvVvvvvvvvVvvh",
            "vvvvVvvvhvvvvvvh",
            "vhvvVvvvhvvvVvvh",
            "vhvvVvvvhvvvVvvv",
            "vhvvvvVvhvvvVvvv",
            "vhvvVvVvhvvvvvvv",
            "vvvvVvvvhvvvVvvv",
            "vhvvVvvvvvvvVvvv",
            "vhvvVvvvhvvvVvhv",
            "vhvvvvvvhvvvVvhv",
            "vhvvVvvvhvvvvvhv",
            "vvvvVvvvhvvvVvvv",
            "vhvvVvvvhvvvVvvv",
        ],
    },
    Art {
        // Light wooden planks for the shrine walls and the bridge.
        name: "shrine_wood",
        palette: &[
            ('w', [186, 140, 90]),
            ('W', [162, 120, 74]),
            ('l', [206, 162, 106]),
            ('d', [120, 84, 50]),
        ],
        rows: [
            "wwwlwwwwwwWwwwww",
            "wWwwwwwwlwwwwwWw",
            "wwwwWwwwwwwwlwww",
            "dddddddddddddddd",
            "wwwwwwwdwwwlwwww",
            "wlwwWwwdwwwwwWww",
            "wwwwwwwdwWwwwwww",
            "dddddddddddddddd",
            "wwWwwwwwwwwwwwdw",
            "wwwwwlwwwWwwwwdw",
            "wWwwwwwwwwwwlwdw",
            "dddddddddddddddd",
            "wwwwwdwwwwwlwwww",
            "wlwwwdwwWwwwwwWw",
            "wwwWwdwwwwwwwwww",
            "dddddddddddddddd",
        ],
    },
    Art {
        name: "bark",
        palette: &[
            ('b', [92, 60, 42]),
            ('B', [72, 46, 32]),
            ('h', [114, 76, 54]),
            ('k', [54, 34, 24]),
        ],
        rows: [
            "bBhbbBkbbhBbbkBb",
            "bBhbbBkbbhBbbkBb",
            "bBbbbBkbhhBbbkBb",
            "kBhbbBbbbhBbbkbb",
            "bBhbkBbbbhbbbkBb",
            "bbhbkBbbbhBbbbBb",
            "bBhbbBbkbhBbbbBh",
            "bBhbbbbkbbBbkbBh",
            "bBbbbBbkbhBbkbBb",
            "kBhbbBbbbhBbkbbb",
            "kBhbbBkbbhbbbbBb",
            "bbhbbBkbbhBbbkBb",
            "bBhbbBkbbbBbbkBb",
            "bBhkbbkbbhBbbkBb",
            "bBhkbBbbbhBhbkbb",
            "bBbkbBbbbhBhbkBb",
        ],
    },
    Art {
        // Rounded clay roof tiles (kawara) running down the roof, each row
        // overlapping the one below it.
        name: "roof_tile",
        palette: &[
            ('h', [112, 122, 148]),
            ('t', [86, 94, 118]),
            ('T', [68, 74, 96]),
            ('d', [42, 46, 62]),
        ],
        rows: [
            "htTdhtTdhtTdhtTd",
            "htTdhtTdhtTdhtTd",
            "htTdhtTdhtTdhtTd",
            "dddddddddddddddd",
            "TdhtTdhtTdhtTdht",
            "TdhtTdhtTdhtTdht",
            "TdhtTdhtTdhtTdht",
            "dddddddddddddddd",
            "htTdhtTdhtTdhtTd",
            "htTdhtTdhtTdhtTd",
            "htTdhtTdhtTdhtTd",
            "dddddddddddddddd",
            "TdhtTdhtTdhtTdht",
            "TdhtTdhtTdhtTdht",
            "TdhtTdhtTdhtTdht",
            "dddddddddddddddd",
        ],
    },
    Art {
        // Clusters of blossoms: two pinks, white petal highlights,
        // and a few deep shadows between flowers.
        name: "sakura",
        palette: &[
            ('p', [246, 182, 206]),
            ('P', [230, 152, 182]),
            ('w', [255, 232, 242]),
            ('d', [198, 112, 148]),
            ('g', [140, 80, 104]),
        ],
        rows: [
            "pPpwpdpPppwpPpgp",
            "wpPpppPpwpdpppPw",
            "pdpPwpgpPpppwPpp",
            "PppppPpwppPdpppP",
            "pwPdpppPpgpppwpp",
            "ppppwPpdpPwpPppd",
            "gpPppppwppppdpwP",
            "pPwpdpPpppPwpppp",
            "ppppPpwpgpppPdpw",
            "wPdpppppPwpdpppP",
            "pppwPpdppppPwpgp",
            "PpgpppwpPdpppppw",
            "pwppPdpppwpPpdpp",
            "ppPwpppgpPppwpPp",
            "dpppwPpppdpwpppP",
            "pPpgpppwPppPpdpw",
        ],
    },
    Art {
        // Calm water with short light ripples.
        name: "water",
        palette: &[
            ('a', [60, 122, 202]),
            ('A', [46, 100, 180]),
            ('l', [92, 152, 226]),
            ('w', [152, 202, 246]),
        ],
        rows: [
            "aaaAaaaaallaaaaa",
            "aaAaaaaaaaaaAaaa",
            "aaaaaaaaaaaaaaaa",
            "alllaaaAaaaaaaaa",
            "aaaawlaaaaaAaaaa",
            "aAaaaaaaaaallaaa",
            "aaaaaaAaaaaawlla",
            "aaaaaaaaaaaaaaaa",
            "aaallaaaaAaaaaaa",
            "aAaaawlaaaaaaaAa",
            "aaaaaaaaaaaaaaaa",
            "aaaaaaaaallaaaaa",
            "laaaAaaaaaawlaaa",
            "aaaaaaaaaaaaaaaA",
            "aaAaaallaaaaaaaa",
            "aaaaaaaawlaaAaaa",
        ],
    },
    Art {
        name: "sand",
        palette: &[
            ('y', [222, 200, 150]),
            ('Y', [205, 183, 132]),
            ('l', [236, 218, 172]),
            ('d', [185, 162, 112]),
        ],
        rows: [
            "yyYyylyyyyYyyyyd",
            "yyyyyyyyldyyyyyy",
            "yYyyydyyyyyyYyly",
            "yyylyyyYyyyyyyyy",
            "dyyyyyyyyylyyyYy",
            "yyyYyylyyyyyyyyy",
            "yyyyyyyyYyydyyly",
            "ylyyydyyyyyyyyyy",
            "yyyyyyylyyyYyyyy",
            "Yyyyyyyyyyyyydyy",
            "yyyldyyyYyylyyyy",
            "yyyyyyyyyyyyyyYy",
            "yYyyyylyyyyyyyyy",
            "yyyyyyyyydyyYyly",
            "ydyyYyyyyyyyyyyy",
            "yyyyyylyyyyyyyyy",
        ],
    },
    Art {
        name: "plaster",
        palette: &[
            ('p', [236, 232, 222]),
            ('P', [222, 218, 206]),
            ('s', [206, 200, 188]),
        ],
        rows: [
            "ppppPppppppPpppp",
            "pPpppppsppppppPp",
            "pppppPppppPppppp",
            "ppsppppppppppspp",
            "pppPpppPpppppppp",
            "pppppppppsppPppp",
            "pPpppsppppppppps",
            "ppppppppPppppppp",
            "pppPpppppppPpppp",
            "psppppPppppppspp",
            "ppppppppppsppppp",
            "ppPppppppppppPpp",
            "pppppsppPppppppp",
            "pPppppppppppspPp",
            "ppppPppsppppppPp",
            "pppppppppPpppppp",
        ],
    },
    Art {
        // Flat modern roof tiles in horizontal rows (the town houses).
        name: "house_roof",
        palette: &[
            ('h', [122, 150, 158]),
            ('t', [92, 116, 126]),
            ('T', [78, 100, 110]),
            ('d', [52, 68, 76]),
        ],
        rows: [
            "hhhhhhhhhhhhhhhh",
            "ttTtttttttttTttt",
            "TtttttTtttTttttt",
            "dddddddddddddddd",
            "hhhhhhhhhhhhhhhh",
            "tttttTttttTttttt",
            "tTtttttttttttTtt",
            "dddddddddddddddd",
            "hhhhhhhhhhhhhhhh",
            "ttTttttTtttttttt",
            "tttttttttTtttTtt",
            "dddddddddddddddd",
            "hhhhhhhhhhhhhhhh",
            "tttTtttttttTtttt",
            "TtttttttTttttttT",
            "dddddddddddddddd",
        ],
    },
    Art {
        name: "road",
        palette: &[
            ('a', [100, 100, 106]),
            ('A', [88, 88, 94]),
            ('l', [120, 120, 126]),
            ('k', [72, 72, 78]),
        ],
        rows: [
            "aaAaaalaaaakaaaa",
            "aaaaAaaaaaaaaAla",
            "alaaaaakaaAaaaaa",
            "aaaaaaaaaaaaaaaa",
            "aAaakaaaalaaaaka",
            "aaaaaaAaaaaaaaaa",
            "aaalaaaaaaaAaaaa",
            "kaaaaaaaakaaaala",
            "aaaaAalaaaaaaaaa",
            "aaaaaaaaaaaaAaaa",
            "aalaaaaAaaakaaaa",
            "aaaaakaaaaaaaaaA",
            "aAaaaaaaalaaaaaa",
            "aaaaaaaaaaaaakaa",
            "aaalaaAaaaaaaaaa",
            "aaaaaaaaaAaalaaa",
        ],
    },
    Art {
        // Grass with fallen sakura petals, for the ground under the trees.
        name: "petal_grass",
        palette: &[
            ('g', [106, 170, 74]),
            ('G', [88, 150, 60]),
            ('l', [128, 192, 90]),
            ('d', [72, 128, 52]),
            ('p', [246, 182, 206]),
            ('w', [255, 230, 240]),
        ],
        rows: [
            "gGglpggGgglggGgg",
            "ggGgggdgGgpgglgG",
            "lgggGgwgggpgGggg",
            "gGgdgglgGgggggdg",
            "gpglgGggggGglggg",
            "GggggpglgdggwgGg",
            "gglgGgGggggGgglg",
            "gdggwgggglgpgGgg",
            "gggGglggGgggdggg",
            "lgGgggdgpgglgggG",
            "ggpgGggglGggwGgg",
            "gGlggggGgggggdgl",
            "ggggdglggGglgpgg",
            "gGgwgggGgpgggGgg",
            "lggGglgggdgGgglg",
            "ggdgggGglgggpggG",
        ],
    },
    Art {
        // Dark evergreen needles (pine trees and lily pads).
        name: "leaves",
        palette: &[
            ('g', [46, 110, 60]),
            ('G', [36, 92, 50]),
            ('l', [66, 138, 78]),
            ('d', [26, 70, 38]),
        ],
        rows: [
            "gGglgdgGglgGgdgg",
            "dgGggGgdgGggGglG",
            "gglGgdgGgglGgdgg",
            "GgdggGglGgdgGggl",
            "gGgldgGgdgGglgGd",
            "lgdGgglgGgdggGgg",
            "gGgGdgGglgGdgGlg",
            "dglggGdgGgglgdGg",
            "gGgdglgGgdGgGglg",
            "GgglGgdglgGgdgGd",
            "gdGgGgglGgdglgGg",
            "lgGgdgGgdgGgGdgl",
            "gGdglgGglgdgGggG",
            "dggGgdglGgGgdglg",
            "gGglgGgdgglGgGgd",
            "GgdgGglgGdgggGlg",
        ],
    },
    Art {
        // Rock with moss growing over the top edge (the cave hill).
        name: "moss_rock",
        palette: &[
            ('m', [90, 140, 70]),
            ('M', [70, 118, 56]),
            ('r', [125, 125, 130]),
            ('R', [105, 105, 112]),
            ('h', [150, 150, 156]),
            ('k', [84, 84, 92]),
        ],
        rows: [
            "mMmmMmmmMmmMmmmM",
            "mmMmmrmMmmmmMrmm",
            "rmmMrrmmrmMmrrmr",
            "rrmrrrrmrrmrrrrr",
            "rrRrrrrrrkrrRrrr",
            "rrrrhrrRrrrrrrrr",
            "Rrrrrrrrrrrhrrkr",
            "rrkkrrrrRrrrrkrr",
            "rrrrkkrrrrrrkrrR",
            "rhrrrrkrrRrrrrrr",
            "rrrRrrrrrrrhrrrr",
            "rrrrrrrhrrrrrRrr",
            "kRrrrrrrrrkkrrrr",
            "rrkrrRrrrkrrrrhr",
            "rrrkrrrrrrrrRrrr",
            "hrrrrrhrrRrrrrrr",
        ],
    },
    Art {
        // A four-pane window: dark frame, bluish glass, a diagonal glint.
        name: "window",
        palette: &[
            ('f', [70, 50, 40]),
            ('g', [120, 150, 190]),
            ('G', [95, 125, 170]),
            ('h', [200, 220, 240]),
        ],
        rows: [
            "ffffffffffffffff",
            "fggghgggfgggghgf",
            "fgghggggfggghggf",
            "fghgggGgfgghgggf",
            "fhgggGggfghgggGf",
            "fgggGgggfhgggGgf",
            "fggGggggfgggGggf",
            "ffffffffffffffff",
            "fgggghggfgggghgf",
            "fggghgggfggghggf",
            "fgghgggGfgghgggf",
            "fghgggGgfghgggGf",
            "fhgggGggfhgggGgf",
            "fgggGgggfgggGggf",
            "fggGggggfggGgggf",
            "ffffffffffffffff",
        ],
    },
    Art {
        // Striped shop awning with a darker hem.
        name: "awning",
        palette: &[
            ('r', [210, 60, 60]),
            ('R', [178, 44, 44]),
            ('w', [245, 240, 230]),
            ('W', [222, 216, 206]),
        ],
        rows: [
            "rrrrwwwwrrrrwwww",
            "rRrrwWwwrRrrwWww",
            "rrrrwwwwrrrrwwww",
            "rrRrwwWwrrRrwwWw",
            "rrrrwwwwrrrrwwww",
            "rRrrwWwwrRrrwWww",
            "rrrrwwwwrrrrwwww",
            "rrrRwwwWrrrRwwwW",
            "rrrrwwwwrrrrwwww",
            "rRrrwWwwrRrrwWww",
            "rrrrwwwwrrrrwwww",
            "rrRrwwWwrrRrwwWw",
            "rrrrwwwwrrrrwwww",
            "RRRRWWWWRRRRWWWW",
            "RRRRWWWWRRRRWWWW",
            "RRRRWWWWRRRRWWWW",
        ],
    },
    Art {
        // Plain concrete for the town's utility poles.
        name: "concrete",
        palette: &[
            ('c', [170, 170, 168]),
            ('C', [155, 155, 153]),
            ('l', [186, 186, 184]),
            ('d', [140, 140, 138]),
        ],
        rows: [
            "cccCcccccccCcccc",
            "cCccccdccccccccl",
            "ccccCcccclccCccc",
            "cccccccccccccccc",
            "cclcccCcccccdccc",
            "cccccccccCcccccc",
            "cCcccdcccccccCcc",
            "ccccccccclcccccc",
            "cccCccccccccCccc",
            "cccccclcccdccccc",
            "cdcccccccccccccc",
            "cccccCcccccCccdc",
            "cccccccclccccccc",
            "ccCcccdcccccCccc",
            "cccccccccccccccl",
            "cclccccCccccdccc",
        ],
    },
    Art {
        // Swirling violet portal surface (it also slides upward over time).
        name: "portal",
        palette: &[
            ('v', [120, 60, 200]),
            ('V', [90, 40, 170]),
            ('p', [200, 120, 255]),
            ('w', [240, 220, 255]),
            ('d', [60, 20, 120]),
        ],
        rows: [
            "vVvpvvVdvvpVvvVv",
            "VvpwpvvVvpwpvVvv",
            "vpwwwpvvpwwwpvvd",
            "vvpwpvVvvpwpvvVv",
            "dvvpvvvdvvpvvvvV",
            "vVvvvVvvVvvvVvpv",
            "vvVdvvpvvvdvvpwp",
            "pvvvvpwpvVvvvvpv",
            "wpvVpwwwpvvVvvvv",
            "pvvvvpwpvvvdvVvv",
            "vVvdvvpvvVvvvvvd",
            "vvvvVvvvvvpvVvvv",
            "dvpvvvVvvpwpvvVv",
            "vpwpvvvdpwwwpvvv",
            "vvpvvVvvvpwpvvdv",
            "VvvvdvvvVvpvvVvv",
        ],
    },
    Art {
        // A glowing red paper lantern: black caps, ribs, warm light inside.
        name: "red_lantern",
        palette: &[
            ('k', [40, 25, 25]),
            ('r', [220, 50, 40]),
            ('R', [190, 35, 30]),
            ('o', [255, 140, 70]),
            ('y', [255, 200, 110]),
        ],
        rows: [
            "kkkkkkkkkkkkkkkk",
            "kkkkkkkkkkkkkkkk",
            "rRrrrRrrrrRrrrRr",
            "rRrorRrorrRrorRr",
            "rRoorRoorrRoorRr",
            "rRoyoRoyorRoyoRr",
            "rRoyyRyyoRyyoyRr",
            "rRyyyRyyyRyyyyRr",
            "rRyyyRyyyRyyyyRr",
            "rRoyyRyyoRyyoyRr",
            "rRoyoRoyorRoyoRr",
            "rRoorRoorrRoorRr",
            "rRrorRrorrRrorRr",
            "rRrrrRrrrrRrrrRr",
            "kkkkkkkkkkkkkkkk",
            "kkkkkkkkkkkkkkkk",
        ],
    },
    Art {
        // Dark red-brown vertical boards for the spirit-world buildings.
        name: "spirit_wood",
        palette: &[
            ('w', [92, 44, 36]),
            ('W', [74, 34, 28]),
            ('l', [112, 58, 46]),
            ('d', [48, 22, 18]),
        ],
        rows: [
            "wWlwdwWlwdwWlwdw",
            "wWlwdwWwwdwWlwdw",
            "wWwwdwWlwdwWwwdw",
            "wWlwdwWlwdwWlwdw",
            "wwlwdwWlwdwwlwdw",
            "wWlwdwWlwdwWlwdw",
            "wWlwdwwlwdwWlwdw",
            "wWlwdwWlwdwWlwdw",
            "wWlwdwWlwdwWlwdw",
            "wWlwdwWlwdwWlwdw",
            "wWwwdwWlwdwWlwdw",
            "wWlwdwWlwdwWwwdw",
            "wWlwdwWlwdwWlwdw",
            "wwlwdwWlwdwWlwdw",
            "wWlwdwWwwdwWlwdw",
            "dddddddddddddddd",
        ],
    },
    Art {
        // Dark blue-grey paving stones for the spirit-world streets.
        name: "spirit_stone",
        palette: &[
            ('s', [70, 68, 88]),
            ('S', [56, 54, 74]),
            ('h', [90, 88, 108]),
            ('m', [36, 34, 50]),
        ],
        rows: [
            "mmmmmmmmmmmmmmmm",
            "msssshssmSSSShSS",
            "mssssssSmSSSSSSS",
            "msSsssssmSShSSSS",
            "msssshssmSSSSSSS",
            "mssssssmmShSSSSS",
            "msssSsssmSSSSSSh",
            "mssssssmmSSSSSSS",
            "mmmmmmmmmmmmmmmm",
            "SSSSmssshsssmSSS",
            "SShSmsssssssmSSh",
            "SSSSmssSssssmSSS",
            "SSSSmssssshsmSSS",
            "hSSSmsssssssmSSS",
            "SSSSmshsssssmSSS",
            "SSSSmsssssSsmSSS",
        ],
    },
    Art {
        // Nearly clear glass: pale, a thin frame and a diagonal glint.
        name: "glass",
        palette: &[
            ('g', [225, 235, 245]),
            ('G', [205, 220, 235]),
            ('h', [255, 255, 255]),
            ('f', [170, 190, 210]),
        ],
        rows: [
            "ffffffffffffffff",
            "fgggggggggggghgf",
            "fggggggggggghggf",
            "fgggggggggghgggf",
            "fggggggggghggggf",
            "fgggggggggggggGf",
            "fggggggggggggGgf",
            "fgggggggggggGggf",
            "fggggggggggGgggf",
            "fgggggggggGggggf",
            "fhgggggggGgggggf",
            "fghggggggggggggf",
            "fgghgggggggggggf",
            "fggghggggggggggf",
            "fgggggggggggggGf",
            "ffffffffffffffff",
        ],
    },
    Art {
        // Kitsune-bi: blue fox-fire flames.
        name: "foxfire",
        palette: &[
            ('b', [60, 120, 255]),
            ('c', [120, 200, 255]),
            ('w', [220, 245, 255]),
            ('d', [30, 60, 200]),
        ],
        rows: [
            "dbbbcbbbdbbcbbbd",
            "bbcbbbcbbbbbbcbb",
            "bcwcbbbbbcbbbbbb",
            "bbcbbbbcbcwcbbcb",
            "dbbbbcbbbbcbbbbb",
            "bbbbcwcbbbbbbcbb",
            "bcbbbcbbbdbbcwcb",
            "bbbbbbbbcbbbbcbb",
            "cbbdbbbcwcbbbbbd",
            "bbbbbbbbcbbbcbbb",
            "bbcbbcbbbbbcwcbb",
            "bcwcbbbbdbbbcbbb",
            "bbcbbbbbbbcbbbbc",
            "dbbbbbcbbcwcbbbb",
            "bbbcbcwcbbcbbbcb",
            "bbbbbbcbbbbbdbbb",
        ],
    },
    Art {
        // Koi scales: orange and white patches, with a few deep red spots.
        name: "koi",
        palette: &[
            ('o', [242, 122, 40]),
            ('O', [222, 96, 30]),
            ('w', [250, 246, 236]),
            ('r', [200, 58, 30]),
        ],
        rows: [
            "wwwoooOoowwwoooo",
            "wwooOoooowwooOoo",
            "wooooOoowwwooooo",
            "ooOoooowwwwoooOo",
            "oooooowwwwoooooo",
            "oOoooowwwoooOooo",
            "oooroooowooooooo",
            "ooOrrooooooooOwo",
            "oooooOoooooowwww",
            "oooooooooooowwww",
            "wwooooOooooowwoo",
            "wwwoooooooOooroo",
            "wwwwooooOooooooo",
            "wwwoooooooooOooo",
            "wwooooOoooowwooo",
            "woooooooooowwwoo",
        ],
    },
    Art {
        // Lantern paper in a dark wooden frame.
        name: "paper",
        palette: &[
            ('p', [255, 238, 192]),
            ('P', [245, 222, 170]),
            ('f', [90, 58, 40]),
        ],
        rows: [
            "ffffffffffffffff",
            "fppppppfpppppppf",
            "fpPppppfppppPppf",
            "fppppppfpppppppf",
            "fpppPppfpppPpppf",
            "fppppppfpppppppf",
            "fppppppfpppppppf",
            "ffffffffffffffff",
            "fppppppfpppppppf",
            "fpppPppfppppppPf",
            "fppppppfpppppppf",
            "fpPppppfppPppppf",
            "fppppppfpppppppf",
            "fppppPpfpppppppf",
            "fppppppfpppppppf",
            "ffffffffffffffff",
        ],
    },
];

fn main() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("textures");
    fs::create_dir_all(&folder).expect("could not create assets/textures");

    let overwrite = std::env::args().any(|argument| argument == "--overwrite");
    for art in TEXTURES {
        let pixels = to_pixels(art); // checks every drawing, even ones we skip
        let path = folder.join(format!("{}.bmp", art.name));
        if path.exists() && !overwrite {
            println!("kept  {} (already exists)", path.display());
            continue;
        }
        write_bmp(&path, &pixels).expect("could not write texture");
        println!("wrote {}", path.display());
    }
}

/// Looks up every letter in the palette. Stops with a clear message if a row
/// is the wrong length or uses a letter the palette doesn't define.
fn to_pixels(art: &Art) -> Vec<[u8; 3]> {
    let mut pixels = Vec::with_capacity(16 * 16);
    for (row_number, row) in art.rows.iter().enumerate() {
        assert_eq!(row.chars().count(), 16, "{}: row {row_number} is not 16 pixels", art.name);
        for letter in row.chars() {
            let color = art
                .palette
                .iter()
                .find(|(key, _)| *key == letter)
                .unwrap_or_else(|| panic!("{}: letter '{letter}' is not in the palette", art.name))
                .1;
            pixels.push(color);
        }
    }
    pixels
}

/// Same 24-bit BMP layout as Framebuffer::save_bmp (16 × 16, rows bottom-up).
fn write_bmp(path: &Path, pixels: &[[u8; 3]]) -> std::io::Result<()> {
    const SIZE: u32 = 16;
    let pixel_bytes = SIZE * SIZE * 3; // 48-byte rows: already a multiple of 4
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&(54 + pixel_bytes).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&54u32.to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&(SIZE as i32).to_le_bytes());
    bytes.extend_from_slice(&(SIZE as i32).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&24u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&pixel_bytes.to_le_bytes());
    bytes.extend_from_slice(&2835i32.to_le_bytes());
    bytes.extend_from_slice(&2835i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    for y in (0..SIZE as usize).rev() {
        for x in 0..SIZE as usize {
            let [r, g, b] = pixels[y * SIZE as usize + x];
            bytes.extend_from_slice(&[b, g, r]);
        }
    }
    fs::File::create(path)?.write_all(&bytes)
}
