/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `table/townname.h`: built-in generator tables, in their original order.
//! Name strings retain their original UTF-8 bytes.
//! Czech tables and generator copyright Petr Baudis 2005 (GPL).

pub(super) const NAME_ORIGINAL_ENGLISH_1: &[&str] = &["Great ", "Little ", "New ", "Fort "];

pub(super) const NAME_ORIGINAL_ENGLISH_2: &[&str] = &[
    "Wr", "B", "C", "Ch", "Br", "D", "Dr", "F", "Fr", "Fl", "G", "Gr", "H", "L", "M", "N", "P",
    "Pr", "Pl", "R", "S", "S", "Sl", "T", "Tr", "W",
];

pub(super) const NAME_ORIGINAL_ENGLISH_3: &[&str] = &["ar", "a", "e", "in", "on", "u", "un", "en"];

pub(super) const NAME_ORIGINAL_ENGLISH_4: &[&str] = &["n", "ning", "ding", "d", "", "t", "fing"];

pub(super) const NAME_ORIGINAL_ENGLISH_5: &[&str] = &[
    "ville", "ham", "field", "ton", "town", "bridge", "bury", "wood", "ford", "hall", "ston",
    "way", "stone", "borough", "ley", "head", "bourne", "pool", "worth", "hill", "well", "hattan",
    "burg",
];

pub(super) const NAME_ORIGINAL_ENGLISH_6: &[&str] = &[
    "-on-sea", " Bay", " Market", " Cross", " Bridge", " Falls", " City", " Ridge", " Springs",
];

pub(super) const NAME_ADDITIONAL_ENGLISH_PREFIX: &[&str] =
    &["Great ", "Little ", "New ", "Fort ", "St. ", "Old "];

pub(super) const NAME_ADDITIONAL_ENGLISH_1A: &[&str] = &[
    "Pen", "Lough", "Stam", "Aber", "Acc", "Ex", "Ax", "Bre", "Cum", "Dun", "Fin", "Inver", "Kin",
    "Mon", "Nan", "Nant", "Pit", "Pol", "Pont", "Strath", "Tre", "Tilly", "Beck", "Canter", "Bath",
    "Liver", "Mal", "Ox", "Bletch", "Maccles", "Grim", "Wind", "Sher", "Gates", "Orp", "Brom",
    "Lewis", "Whit", "White", "Worm", "Tyne", "Avon", "Stan",
];

pub(super) const NAME_ADDITIONAL_ENGLISH_1B1: &[&str] = &[
    "Wr", "B", "C", "Ch", "Br", "D", "Dr", "F", "Fr", "Fl", "G", "Gr", "H", "L", "M", "N", "P",
    "Pr", "Pl", "R", "S", "S", "Sl", "T", "Tr", "W",
];

pub(super) const NAME_ADDITIONAL_ENGLISH_1B2: &[&str] = &[
    "ar", "a", "e", "in", "on", "u", "o", "ee", "es", "ea", "un", "en",
];

pub(super) const NAME_ADDITIONAL_ENGLISH_1B3A: &[&str] = &["n", "d", "", "t", "", ""];

pub(super) const NAME_ADDITIONAL_ENGLISH_1B3B: &[&str] = &["ning", "ding", "fing"];

pub(super) const NAME_ADDITIONAL_ENGLISH_2: &[&str] = &[
    "ville", "ham", "field", "ton", "town", "borough", "bridge", "bury", "wood", "ditch", "ford",
    "hall", "dean", "leigh", "dore", "ston", "stow", "church", "wich", "low", "way", "stone",
    "minster", "ley", "head", "bourne", "pool", "worth", "hill", "well", "hattan", "burg", "berg",
    "burgh", "port", "stoke", "haven", "stable", "stock", "side", "brook", "don", "den", "down",
    "nor", "grove", "combe", "by", "say", "ney", "chester", "dale", "ness", "shaw", "thwaite",
];

pub(super) const NAME_ADDITIONAL_ENGLISH_3: &[&str] = &[
    "-on-sea", " Bay", " Market", " Beeches", " Common", " Park", " Heath", " Marsh", " Green",
    " Castle", " End", " Rivers", " Cross", " Bridge", " Falls", " City", " Ridge", " Springs",
];

pub(super) const NAME_AUSTRIAN_A1: &[&str] =
    &["Bad ", "Deutsch ", "Gross ", "Klein ", "Markt ", "Maria "];

pub(super) const NAME_AUSTRIAN_A2: &[&str] = &[
    "Aus", "Alten", "Braun", "Vösl", "Mittern", "Nuss", "Neu", "Walters", "Breiten", "Eisen",
    "Feld", "Mittern", "Gall", "Obern", "Grat", "Heiligen", "Hof", "Holla", "Stein", "Eber",
    "Eggen", "Enzers", "Frauen", "Herren", "Hof", "Hütt", "Kaisers", "Königs", "Knittel", "Lang",
    "Ober", "Ollers", "Pfaffen", "Potten", "Salz", "Schwarz", "Stocker", "Unter", "Utten", "Vösen",
    "Vill", "Weissen",
];

pub(super) const NAME_AUSTRIAN_A3: &[&str] = &[
    "see", "bach", "dorf", "ach", "stein", "hofen", "au", "ach", "kirch", "kirchen", "kreuz",
    "brunn", "siedl", "markt", "wang", "haag",
];

pub(super) const NAME_AUSTRIAN_A4: &[&str] = &[
    "Bruck",
    "Brunn",
    "Gams",
    "Grein",
    "Ried",
    "Faak",
    "Zell",
    "Spital",
    "Kirchberg",
    "Saal",
    "Taferl",
    "Wald",
];

pub(super) const NAME_AUSTRIAN_A5: &[&str] = &["St. ", "Sankt "];

pub(super) const NAME_AUSTRIAN_A6: &[&str] = &[
    "Aegyd",
    "Andrä",
    "Georgen",
    "Jakob",
    "Johann",
    "Leonhard",
    "Marein",
    "Lorenzen",
    "Margarethen",
    "Martin",
    "Michael",
    "Nikolai",
    "Oswald",
    "Peter",
    "Pölten",
    "Stefan",
    "Stephan",
    "Thomas",
    "Veit",
    "Wolfgang",
];

pub(super) const NAME_AUSTRIAN_F1: &[&str] = &[" an der ", " ob der "];

pub(super) const NAME_AUSTRIAN_F2: &[&str] = &[
    "Donau", "Steyr", "Lafnitz", "Leitha", "Thaya", "Gail", "Drau", "Salzach", "Ybbs", "Traisen",
    "Enns", "Mur", "Ill",
];

pub(super) const NAME_AUSTRIAN_B1: &[&str] = &[" am "];

pub(super) const NAME_AUSTRIAN_B2: &[&str] = &[
    "Brenner",
    "Dachstein",
    "Gebirge",
    "Grossglockner",
    "Hausruck",
    "Semmering",
    "Wagram",
    "Wechsel",
    "Wilden Kaiser",
    "Ziller",
];

pub(super) const NAME_GERMAN_REAL: &[&str] = &[
    "Berlin",
    "Bonn",
    "Bremen",
    "Cottbus",
    "Chemnitz",
    "Dortmund",
    "Dresden",
    "Erfurt",
    "Erlangen",
    "Essen",
    "Fulda",
    "Gera",
    "Kassel",
    "Kiel",
    "Köln",
    "Lübeck",
    "Magdeburg",
    "München",
    "Potsdam",
    "Stuttgart",
    "Wiesbaden",
];

pub(super) const NAME_GERMAN_PRE: &[&str] = &["Bad ", "Klein ", "Neu "];

pub(super) const NAME_GERMAN_1: &[&str] = &[
    "Alb", "Als", "Ander", "Arns", "Bruns", "Bam", "Biele", "Cloppen", "Co", "Duis", "Düssel",
    "Dannen", "Elb", "Els", "Elster", "Eichen", "Ems", "Fahr", "Falken", "Flens", "Frank", "Frei",
    "Freuden", "Fried", "Fürsten", "Hahn", "Ham", "Harz", "Heidel", "Hers", "Herz", "Holz",
    "Hildes", "Inns", "Ilsen", "Ingols", "Kel", "Kies", "Korn", "Kor", "Kreuz", "Kulm", "Langen",
    "Lim", "Lohr", "Lüne", "Mel", "Michels", "Mühl", "Naum", "Nest", "Nord", "Nort", "Nien",
    "Nidda", "Nieder", "Nürn", "Ober", "Offen", "Osna", "Olden", "Ols", "Oranien", "Pader",
    "Quedlin", "Quer", "Ravens", "Regens", "Rott", "Ros", "Rüssels", "Saal", "Saar", "Salz",
    "Schöne", "Schwein", "Sonder", "Sonnen", "Stein", "Strals", "Straus", "Süd", "Ton", "Unter",
    "Ur", "Vor", "Wald", "War", "Wert", "Wester", "Witten", "Wolfs", "Würz",
];

pub(super) const NAME_GERMAN_2: &[&str] = &[
    "bach", "berg", "brück", "brücken", "burg", "dorf", "feld", "furt", "hausen", "haven", "heim",
    "horst", "mund", "münster", "stadt", "wald",
];

pub(super) const NAME_GERMAN_3_AN_DER: &[&str] = &[" an der "];

pub(super) const NAME_GERMAN_3_AM: &[&str] = &[" am "];

pub(super) const NAME_GERMAN_4_AN_DER: &[&str] = &["Oder", "Spree", "Donau", "Saale", "Elbe"];

pub(super) const NAME_GERMAN_4_AM: &[&str] = &["Main"];

pub(super) const NAME_SPANISH_REAL: &[&str] = &[
    "Caracas",
    "Maracay",
    "Maracaibo",
    "Valencia",
    "El Dorado",
    "Morrocoy",
    "Cata",
    "Cataito",
    "Ciudad Bolívar",
    "Barquisimeto",
    "Mérida",
    "Puerto Ordaz",
    "Santa Elena",
    "San Juan",
    "San Luis",
    "San Rafael",
    "Santiago",
    "Barcelona",
    "Barinas",
    "San Cristóbal",
    "San Francisco",
    "San Martín",
    "Guayana",
    "San Carlos",
    "El Limón",
    "Coro",
    "Coro Coro",
    "Puerto Ayacucho",
    "Elorza",
    "Arismendi",
    "Trujillo",
    "Carúpano",
    "Anaco",
    "Lima",
    "Cuzco",
    "Iquitos",
    "Callao",
    "Huacho",
    "Camaná",
    "Puerto Chala",
    "Santa Cruz",
    "Quito",
    "Cuenca",
    "Medellín",
    "Tulcán",
    "Esmeraldas",
    "Ibarra",
    "San Lorenzo",
    "Macas",
    "Moraña",
    "Machala",
    "Zamora",
    "Latacunga",
    "Tena",
    "Cochabamba",
    "Ascensión",
    "Magdalena",
    "Santa Ana",
    "Manoa",
    "Sucre",
    "Oruro",
    "Uyuni",
    "Potosí",
    "Tupiza",
    "La Quiaca",
    "Yacuíba",
    "San Borja",
    "Fuerte Olimpo",
    "Fortín Esteros",
    "Campo Grande",
    "Bogotá",
    "El Banco",
    "Zaragoza",
    "Neiva",
    "Mariano",
    "Cali",
    "La Palma",
    "Andoas",
    "Barranca",
    "Montevideo",
    "Valdivia",
    "Arica",
    "Temuco",
    "Tocopilla",
    "Mendoza",
    "Santa Rosa",
];

pub(super) const NAME_FRENCH_REAL: &[&str] = &[
    "Agincourt",
    "Lille",
    "Dinan",
    "Aubusson",
    "Rodez",
    "Bergerac",
    "Bordeaux",
    "Bayonne",
    "Montpellier",
    "Montélimar",
    "Valence",
    "Digne",
    "Nice",
    "Cannes",
    "St. Tropez",
    "Marseille",
    "Narbonne",
    "Sète",
    "Aurillac",
    "Guéret",
    "Le Creusot",
    "Nevers",
    "Auxerre",
    "Versailles",
    "Meaux",
    "Châlons",
    "Compiègne",
    "Metz",
    "Chaumont",
    "Langres",
    "Bourg",
    "Lyon",
    "Vienne",
    "Grenoble",
    "Toulon",
    "Rennes",
    "Le Mans",
    "Angers",
    "Nantes",
    "Châteauroux",
    "Orléans",
    "Lisieux",
    "Cherbourg",
    "Morlaix",
    "Cognac",
    "Agen",
    "Tulle",
    "Blois",
    "Troyes",
    "Charolles",
    "Toulouse",
    "Chambéry",
    "Tours",
    "St. Brieuc",
    "St. Malo",
    "La Rochelle",
    "St. Flour",
    "Le Puy",
    "Vichy",
    "St. Valery",
    "Beaujolais",
    "Reims",
    "Albi",
    "Paris",
    "Biarritz",
    "Béziers",
    "Nîmes",
    "Chamonix",
    "Angoulème",
    "Alençon",
];

pub(super) const NAME_SILLY_1: &[&str] = &[
    "Binky", "Blubber", "Bumble", "Crinkle", "Crusty", "Dangle", "Dribble", "Flippety", "Google",
    "Muffin", "Nosey", "Pinker", "Quack", "Rumble", "Sleepy", "Sliggles", "Snooze", "Teddy",
    "Tinkle", "Twister", "Pinker", "Hippo", "Itchy", "Jelly", "Jingle", "Jolly", "Kipper", "Lazy",
    "Frogs", "Mouse", "Quack", "Cheeky", "Lumpy", "Grumpy", "Mangle", "Fiddle", "Slugs", "Noodles",
    "Poodles", "Shiver", "Rumble", "Pixie", "Puddle", "Riddle", "Rattle", "Rickety", "Waffle",
    "Sagging", "Sausage", "Egg", "Sleepy", "Scatter", "Scramble", "Silly", "Simple", "Trickle",
    "Slippery", "Slimey", "Slumber", "Soggy", "Sliggles", "Splutter", "Sulky", "Swindle", "Swivel",
    "Tasty", "Tangle", "Toggle", "Trotting", "Tumble", "Snooze", "Water", "Windy", "Amble",
    "Bubble", "Cheery", "Cheese", "Cockle", "Cracker", "Crumple", "Teddy", "Evil", "Fairy",
    "Falling", "Fishy", "Fizzle", "Frosty", "Griddle",
];

pub(super) const NAME_SILLY_2: &[&str] = &[
    "ton", "bury", "bottom", "ville", "well", "weed", "worth", "wig", "wick", "wood", "pool",
    "head", "burg", "gate", "bridge",
];

pub(super) const NAME_SWEDISH_1: &[&str] = &["Gamla ", "Lilla ", "Nya ", "Stora "];

pub(super) const NAME_SWEDISH_2: &[&str] = &[
    "Boll", "Bor", "Ed", "En", "Erik", "Es", "Fin", "Fisk", "Grön", "Hag", "Halm", "Karl", "Kram",
    "Kung", "Land", "Lid", "Lin", "Mal", "Malm", "Marie", "Ner", "Norr", "Oskar", "Sand", "Skog",
    "Stock", "Stor", "Ström", "Sund", "Söder", "Tall", "Tratt", "Troll", "Upp", "Var", "Väster",
    "Ängel", "Öster",
];

pub(super) const NAME_SWEDISH_2A: &[&str] = &[
    "B", "Br", "D", "Dr", "Dv", "F", "Fj", "Fl", "Fr", "G", "Gl", "Gn", "Gr", "H", "J", "K", "Kl",
    "Kn", "Kr", "Kv", "L", "M", "N", "P", "Pl", "Pr", "R", "S", "Sk", "Skr", "Sl", "Sn", "Sp",
    "Spr", "St", "Str", "Sv", "T", "Tr", "Tv", "V", "Vr",
];

pub(super) const NAME_SWEDISH_2B: &[&str] = &["a", "e", "i", "o", "u", "y", "å", "ä", "ö"];

pub(super) const NAME_SWEDISH_2C: &[&str] = &[
    "ck", "d", "dd", "g", "gg", "l", "ld", "m", "n", "nd", "ng", "nn", "p", "pp", "r", "rd", "rk",
    "rp", "rr", "rt", "s", "sk", "st", "t", "tt", "v",
];

pub(super) const NAME_SWEDISH_3: &[&str] = &[
    "arp", "berg", "boda", "borg", "bro", "bukten", "by", "byn", "fors", "hammar", "hamn", "holm",
    "hus", "hättan", "kulle", "köping", "lund", "löv", "sala", "skrona", "slätt", "spång", "stad",
    "sund", "svall", "svik", "såker", "udde", "valla", "viken", "älv", "ås",
];

pub(super) const NAME_DUTCH_1: &[&str] = &[
    "Nieuw ", "Oud ", "Groot ", "Zuid ", "Noord ", "Oost ", "West ", "Klein ",
];

pub(super) const NAME_DUTCH_2: &[&str] = &[
    "Hoog", "Laag", "Zuider", "Zuid", "Ooster", "Oost", "Wester", "West", "Hoofd", "Midden",
    "Eind", "Amster", "Amstel", "Dord", "Rotter", "Haar", "Til", "Enk", "Dok", "Veen", "Leidsch",
    "Lely", "En", "Kaats", "U", "Maas", "Mar", "Bla", "Al", "Alk", "Eer", "Drie", "Ter", "Groes",
    "Goes", "Soest", "Coe", "Uit", "Zwaag", "Hellen", "Slie", "IJ", "Grubben", "Groen", "Lek",
    "Ridder", "Schie", "Olde", "Roose", "Haar", "Til", "Loos", "Hil",
];

pub(super) const NAME_DUTCH_3: &[&str] = &[
    "Drog", "Nat", "Valk", "Bob", "Dedem", "Kollum", "Best", "Hoend", "Leeuw", "Graaf", "Uithuis",
    "Purm", "Hard", "Hell", "Werk", "Spijk", "Vink", "Wams", "Heerhug", "Koning",
];

pub(super) const NAME_DUTCH_4: &[&str] = &["e", "er", "el", "en", "o", "s"];

pub(super) const NAME_DUTCH_5: &[&str] = &[
    "stad", "vorst", "dorp", "dam", "beek", "doorn", "zijl", "zijlen", "lo", "muiden", "meden",
    "vliet", "nisse", "daal", "vorden", "vaart", "mond", "zaal", "water", "duinen", "heuvel",
    "geest", "kerk", "meer", "maar", "hoorn", "rade", "wijk", "berg", "heim", "sum", "richt",
    "burg", "recht", "drecht", "trecht", "tricht", "dricht", "lum", "rum", "halen", "oever",
    "wolde", "veen", "hoven", "gast", "kum", "hage", "dijk", "zwaag", "pomp", "huizen", "bergen",
    "schede", "mere", "end",
];

pub(super) const NAME_FINNISH_REAL: &[&str] = &[
    "Aijala",
    "Kisko",
    "Espoo",
    "Helsinki",
    "Tapiola",
    "Järvelä",
    "Lahti",
    "Kotka",
    "Hamina",
    "Loviisa",
    "Kouvola",
    "Tampere",
    "Oulu",
    "Salo",
    "Malmi",
    "Pelto",
    "Koski",
    "Iisalmi",
    "Raisio",
    "Taavetti",
    "Joensuu",
    "Imatra",
    "Tapanila",
    "Pasila",
    "Turku",
    "Kupittaa",
    "Vaasa",
    "Pori",
    "Rauma",
    "Kolari",
    "Lieksa",
];

pub(super) const NAME_FINNISH_1: &[&str] = &[
    "Hiekka", "Haapa", "Mylly", "Sauna", "Uusi", "Vanha", "Kesä", "Kuusi", "Pelto", "Tuomi",
    "Terva", "Olki", "Heinä", "Seinä", "Rova", "Koivu", "Kokko", "Mänty", "Pihlaja", "Petäjä",
    "Kielo", "Kauha", "Viita", "Kivi", "Riihi", "Ääne", "Niini",
];

pub(super) const NAME_FINNISH_2: &[&str] = &[
    "Lappeen",
    "Lohjan",
    "Savon",
    "Lapin",
    "Pitäjän",
    "Martin",
    "Kuusan",
    "Kemi",
    "Keri",
    "Hämeen",
    "Kangas",
];

pub(super) const NAME_FINNISH_3: &[&str] = &[
    "harju", "linna", "järvi", "kallio", "mäki", "nummi", "joki", "kylä", "lampi", "lahti",
    "metsä", "suo", "laakso", "niitty", "luoto", "hovi", "ranta", "koski", "salo",
];

pub(super) const NAME_POLISH_1_M: &[&str] = &[
    "Wielki ", "Mały ", "Zły ", "Dobry ", "Nowy ", "Stary ", "Złoty ", "Zielony ", "Biały ",
    "Modry ", "Dębowy ",
];

pub(super) const NAME_POLISH_1_F: &[&str] = &[
    "Wielka ", "Mała ", "Zła ", "Dobra ", "Nowa ", "Stara ", "Złota ", "Zielona ", "Biała ",
    "Modra ", "Dębowa ",
];

pub(super) const NAME_POLISH_1_N: &[&str] = &[
    "Wielkie ", "Małe ", "Złe ", "Dobre ", "Nowe ", "Stare ", "Złote ", "Zielone ", "Białe ",
    "Modre ", "Dębowe ",
];

pub(super) const NAME_POLISH_2_O: &[&str] = &[
    "Frombork",
    "Gniezno",
    "Olsztyn",
    "Toruń",
    "Bydgoszcz",
    "Terespol",
    "Kraków",
    "Poznań",
    "Wrocław",
    "Katowice",
    "Cieszyn",
    "Bytom",
    "Frombork",
    "Hel",
    "Konin",
    "Lublin",
    "Malbork",
    "Sopot",
    "Sosnowiec",
    "Gdańsk",
    "Gdynia",
    "Sieradz",
    "Sandomierz",
    "Szczyrk",
    "Szczytno",
    "Szczecin",
    "Zakopane",
    "Szklarska Poręba",
    "Bochnia",
    "Golub-Dobrzyń",
    "Chojnice",
    "Ostrowiec",
    "Otwock",
    "Wolsztyn",
];

pub(super) const NAME_POLISH_2_M: &[&str] = &[
    "Jarocin",
    "Gogolin",
    "Tomaszów",
    "Piotrków",
    "Lidzbark",
    "Rypin",
    "Radzymin",
    "Wołomin",
    "Pruszków",
    "Olsztynek",
    "Rypin",
    "Cisek",
    "Krotoszyn",
    "Stoczek",
    "Lubin",
    "Lubicz",
    "Milicz",
    "Targ",
    "Ostrów",
    "Ozimek",
    "Puck",
    "Rzepin",
    "Siewierz",
    "Stargard",
    "Starogard",
    "Turek",
    "Tymbark",
    "Wolsztyn",
    "Strzepcz",
    "Strzebielin",
    "Sochaczew",
    "Grębocin",
    "Gniew",
    "Lubliniec",
    "Lubasz",
    "Lutomiersk",
    "Niemodlin",
    "Przeworsk",
    "Ursus",
    "Tyczyn",
    "Sztum",
    "Szczebrzeszyn",
    "Wolin",
    "Wrzeszcz",
    "Zgierz",
    "Zieleniec",
    "Drobin",
    "Garwolin",
];

pub(super) const NAME_POLISH_2_F: &[&str] = &[
    "Szprotawa",
    "Pogorzelica",
    "Motława",
    "Lubawa",
    "Nidzica",
    "Kruszwica",
    "Bierawa",
    "Brodnica",
    "Chojna",
    "Krzepica",
    "Ruda",
    "Rumia",
    "Tuchola",
    "Trzebinia",
    "Ustka",
    "Warszawa",
    "Bobowa",
    "Dukla",
    "Krynica",
    "Murowana",
    "Niemcza",
    "Zaspa",
    "Zawoja",
    "Wola",
    "Limanowa",
    "Rabka",
    "Skawina",
    "Pilawa",
];

pub(super) const NAME_POLISH_2_N: &[&str] = &[
    "Lipsko",
    "Pilzno",
    "Przodkowo",
    "Strzelno",
    "Leszno",
    "Jaworzno",
    "Choszczno",
    "Mogilno",
    "Luzino",
    "Miasto",
    "Dziadowo",
    "Kowalewo",
    "Legionowo",
    "Miastko",
    "Zabrze",
    "Zawiercie",
    "Kochanowo",
    "Miechucino",
    "Mirachowo",
    "Robakowo",
    "Kosakowo",
    "Borne",
    "Braniewo",
    "Sulinowo",
    "Chmielno",
    "Jastrzębie",
    "Gryfino",
    "Koronowo",
    "Lubichowo",
    "Opoczno",
];

pub(super) const NAME_POLISH_3_M: &[&str] = &[
    " Wybudowanie",
    " Świętokrzyski",
    " Górski",
    " Morski",
    " Zdrój",
    " Wody",
    " Bajoro",
    " Krajeński",
    " Śląski",
    " Mazowiecki",
    " Pomorski",
    " Wielki",
    " Maly",
    " Warmiński",
    " Mazurski",
    " Mniejszy",
    " Większy",
    " Górny",
    " Dolny",
    " Wielki",
    " Stary",
    " Nowy",
    " Wielkopolski",
    " Wzgórze",
    " Mosty",
    " Kujawski",
    " Małopolski",
    " Podlaski",
    " Lesny",
];

pub(super) const NAME_POLISH_3_F: &[&str] = &[
    " Wybudowanie",
    " Świętokrzyska",
    " Górska",
    " Morska",
    " Zdrój",
    " Woda",
    " Bajoro",
    " Krajeńska",
    " Śląska",
    " Mazowiecka",
    " Pomorska",
    " Wielka",
    " Mała",
    " Warmińska",
    " Mazurska",
    " Mniejsza",
    " Większa",
    " Górna",
    " Dolna",
    " Wielka",
    " Stara",
    " Nowa",
    " Wielkopolska",
    " Wzgórza",
    " Mosty",
    " Kujawska",
    " Malopolska",
    " Podlaska",
    " Leśna",
];

pub(super) const NAME_POLISH_3_N: &[&str] = &[
    " Wybudowanie",
    " Świetokrzyskie",
    " Górskie",
    " Morskie",
    " Zdrój",
    " Wody",
    " Bajoro",
    " Krajeńskie",
    " Śląskie",
    " Mazowieckie",
    " Pomorskie",
    " Wielkie",
    " Małe",
    " Warmińskie ",
    " Mazurskie ",
    " Mniejsze",
    " Większe",
    " Górne",
    " Dolne",
    " Wielkie",
    " Stare",
    " Nowe",
    " Wielkopolskie",
    " Wzgórze",
    " Mosty",
    " Kujawskie",
    " Małopolskie",
    " Podlaskie",
    " Leśne",
];

pub(super) const NAME_CZECH_REAL: &[&str] = &[
    "Aš",
    "Benešov",
    "Beroun",
    "Bezdružice",
    "Blansko",
    "Břeclav",
    "Brno",
    "Bruntál",
    "Česká Lípa",
    "České Budějovice",
    "Český Krumlov",
    "Děčín",
    "Domažlice",
    "Dubí",
    "Frýdek-Místek",
    "Havlíčkův Brod",
    "Hodonín",
    "Hradec Králové",
    "Humpolec",
    "Cheb",
    "Chomutov",
    "Chrudim",
    "Jablonec nad Nisou",
    "Jeseník",
    "Jičín",
    "Jihlava",
    "Jindřichův Hradec",
    "Karlovy Vary",
    "Karviná",
    "Kladno",
    "Klatovy",
    "Kolín",
    "Kosmonosy",
    "Kroměříž",
    "Kutná Hora",
    "Liberec",
    "Litoměřice",
    "Louny",
    "Manětín",
    "Mělník",
    "Mladá Boleslav",
    "Most",
    "Náchod",
    "Nový Jičín",
    "Nymburk",
    "Olomouc",
    "Opava",
    "Oráčov",
    "Ostrava",
    "Pardubice",
    "Pelhřimov",
    "Polžice",
    "Písek",
    "Plzeň",
    "Praha",
    "Prachatice",
    "Přerov",
    "Příbram",
    "Prostějov",
    "Rakovník",
    "Rokycany",
    "Rudná",
    "Rychnov nad Kněžnou",
    "Semily",
    "Sokolov",
    "Strakonice",
    "Středokluky",
    "Šumperk",
    "Svitavy",
    "Tábor",
    "Tachov",
    "Teplice",
    "Třebíč",
    "Trutnov",
    "Uherské Hradiště",
    "Ústí nad Labem",
    "Ústí nad Orlicí",
    "Vsetín",
    "Vyškov",
    "Žďár nad Sázavou",
    "Zlín",
    "Znojmo",
];

// [CzechGender][CzechPattern]: suffixes for adjectives.
pub(super) const NAME_CZECH_PATMOD: &[[&str; 3]] = &[
    /* CZG_SMASC */ ["í", "ý", "uv"],
    /* CZG_SFEM */ ["í", "á", "ova"],
    /* CZG_SNEUT */ ["í", "é", "ovo"],
    /* CZG_PMASC */ ["í", "é", "ovy"],
    /* CZG_PFEM */ ["í", "é", "ovy"],
    /* CZG_PNEUT */ ["í", "á", "ova"],
];

// Some items which should be common are doubled.
pub(super) const NAME_CZECH_ADJ: &[CzechNameAdj] = &[
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Horn",
    },
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Horn",
    },
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Doln",
    },
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Doln",
    },
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Předn",
    },
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Zadn",
    },
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Kosteln",
    },
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Havran",
    },
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Říčn",
    },
    CzechNameAdj {
        pattern: CZP_JARNI,
        choose: CZC_ANY,
        name: "Jezern",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Velk",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Velk",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Mal",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Mal",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Vysok",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Česk",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Moravsk",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Slováck",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Slezsk",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Uhersk",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Star",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Star",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Nov",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Nov",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Mlad",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Královsk",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Kamenn",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Cihlov",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CZC_ANY,
        name: "Divn",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Červen",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Červen",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Červen",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Zelen",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Žlut",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Siv",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Šed",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Bíl",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Bíl",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Modr",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Růžov",
    },
    CzechNameAdj {
        pattern: CZP_MLADY,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Čern",
    },
    CzechNameAdj {
        pattern: CZP_PRIVL,
        choose: CZC_ANY,
        name: "Král",
    },
    CzechNameAdj {
        pattern: CZP_PRIVL,
        choose: CZC_ANY,
        name: "Jan",
    },
    CzechNameAdj {
        pattern: CZP_PRIVL,
        choose: CZC_ANY,
        name: "Karl",
    },
    CzechNameAdj {
        pattern: CZP_PRIVL,
        choose: CZC_ANY,
        name: "Kryštof",
    },
    CzechNameAdj {
        pattern: CZP_PRIVL,
        choose: CZC_ANY,
        name: "Jiřík",
    },
    CzechNameAdj {
        pattern: CZP_PRIVL,
        choose: CZC_ANY,
        name: "Petr",
    },
    CzechNameAdj {
        pattern: CZP_PRIVL,
        choose: CZC_ANY,
        name: "Sud",
    },
];

// Considered a stem for choose/allow matching purposes.
pub(super) const NAME_CZECH_SUBST_FULL: &[CzechNameSubst] = &[
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Sedlec",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Brod",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Brod",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CZA_ALL,
        choose: CzechChooseFlags::EMPTY,
        name: "Úval",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Žďár",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Smrk",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Hora",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Lhota",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Lhota",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Hlava",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Lípa",
    },
    CzechNameSubst {
        gender: CZG_SNEUT,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Pole",
    },
    CzechNameSubst {
        gender: CZG_SNEUT,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Údolí",
    },
    CzechNameSubst {
        gender: CZG_PMASC,
        allow: CZA_ALL,
        choose: CzechChooseFlags::EMPTY,
        name: "Úvaly",
    },
    CzechNameSubst {
        gender: CZG_PFEM,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Luka",
    },
    CzechNameSubst {
        gender: CZG_PNEUT,
        allow: CZA_ALL,
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Pole",
    },
];

// TODO: More stems needed. --pasky
pub(super) const NAME_CZECH_SUBST_STEM: &[CzechNameSubst] = &[
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Kostel",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Klášter",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Lhot",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Lhot",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Hur",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        choose: CzechChooseFlags::EMPTY,
        name: "Sedl",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short).with(
            CzechAllowFlags::from_element(CzechAllowFlag::Middle)
                .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        ),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Hrad",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle),
        choose: CzechChooseFlags::EMPTY,
        name: "Pras",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle),
        choose: CzechChooseFlags::EMPTY,
        name: "Baž",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle),
        choose: CzechChooseFlags::EMPTY,
        name: "Tes",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle),
        choose: CzechChooseFlags::EMPTY,
        name: "Už",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Postfix),
        name: "Bř",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        choose: CzechChooseFlags::EMPTY,
        name: "Vod",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        choose: CzechChooseFlags::EMPTY,
        name: "Jan",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Prach",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Kunr",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Strak",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Vít",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Vyš",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Žat",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Žer",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Střed",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Harv",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Pruh",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Tach",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Písn",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Jin",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Jes",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Jar",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Sok",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Hod",
    },
    CzechNameSubst {
        gender: CZG_NFREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Net",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Praž",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Nerat",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Kral",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::EMPTY,
        name: "Hut",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::NoPostfix),
        name: "Pan",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short).with(
            CzechAllowFlags::from_element(CzechAllowFlag::Middle)
                .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        ),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::NoPostfix),
        name: "Odstřed",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short).with(
            CzechAllowFlags::from_element(CzechAllowFlag::Middle)
                .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        ),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Mrat",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Colour),
        name: "Hlav",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Middle)),
        choose: CzechChooseFlags::EMPTY,
        name: "Měř",
    },
    CzechNameSubst {
        gender: CZG_FREE,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        choose: CzechChooseFlags::EMPTY,
        name: "Lip",
    },
];

// Optional postfix inserted between stem and ending.
pub(super) const NAME_CZECH_SUBST_POSTFIX: &[&str] =
    &["av", "an", "at", "ov", "on", "ot", "ev", "en", "et"];

// Both neuter genders must be at the end of this array!
pub(super) const NAME_CZECH_SUBST_ENDING: &[CzechNameSubst] = &[
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Middle)),
        choose: CZC_ANY,
        name: "ec",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Middle)),
        choose: CZC_ANY,
        name: "ín",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short).with(
            CzechAllowFlags::from_element(CzechAllowFlag::Middle)
                .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        ),
        choose: CZC_ANY,
        name: "ov",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        choose: CZC_ANY,
        name: "kov",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Postfix),
        name: "ín",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Postfix),
        name: "ník",
    },
    CzechNameSubst {
        gender: CZG_SMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CZC_ANY,
        name: "burk",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short),
        choose: CZC_ANY,
        name: "ka",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle),
        choose: CZC_ANY,
        name: "inka",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Middle),
        choose: CZC_ANY,
        name: "ná",
    },
    CzechNameSubst {
        gender: CZG_SFEM,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CZC_ANY,
        name: "ava",
    },
    CzechNameSubst {
        gender: CZG_PMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CzechChooseFlags::from_element(CzechChooseFlag::Postfix),
        name: "íky",
    },
    CzechNameSubst {
        gender: CZG_PMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CZC_ANY,
        name: "upy",
    },
    CzechNameSubst {
        gender: CZG_PMASC,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CZC_ANY,
        name: "olupy",
    },
    CzechNameSubst {
        gender: CZG_PFEM,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CZC_ANY,
        name: "avy",
    },
    CzechNameSubst {
        gender: CZG_PFEM,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short).with(
            CzechAllowFlags::from_element(CzechAllowFlag::Middle)
                .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        ),
        choose: CZC_ANY,
        name: "ice",
    },
    CzechNameSubst {
        gender: CZG_PFEM,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short).with(
            CzechAllowFlags::from_element(CzechAllowFlag::Middle)
                .with(CzechAllowFlags::from_element(CzechAllowFlag::Long)),
        ),
        choose: CZC_ANY,
        name: "ičky",
    },
    CzechNameSubst {
        gender: CZG_PNEUT,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Middle)),
        choose: CZC_ANY,
        name: "na",
    },
    CzechNameSubst {
        gender: CZG_SNEUT,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Short)
            .with(CzechAllowFlags::from_element(CzechAllowFlag::Middle)),
        choose: CZC_ANY,
        name: "no",
    },
    CzechNameSubst {
        gender: CZG_SNEUT,
        allow: CzechAllowFlags::from_element(CzechAllowFlag::Long),
        choose: CZC_ANY,
        name: "iště",
    },
];

pub(super) const NAME_CZECH_SUFFIX: &[&str] = &[
    "nad Cidlinou",
    "nad Dyjí",
    "nad Jihlavou",
    "nad Labem",
    "nad Lesy",
    "nad Moravou",
    "nad Nisou",
    "nad Odrou",
    "nad Ostravicí",
    "nad Sázavou",
    "nad Vltavou",
    "pod Pradědem",
    "pod Radhoštěm",
    "pod Řípem",
    "pod Sněžkou",
    "pod Špičákem",
    "pod Sedlem",
    "v Čechách",
    "na Moravě",
];

pub(super) const NAME_ROMANIAN_REAL: &[&str] = &[
    "Adjud",
    "Alba Iulia",
    "Alexandria",
    "Babadag",
    "Bacău",
    "Baia Mare",
    "Băile Herculane",
    "Băilești",
    "Bârlad",
    "Bicaz",
    "Bistrița",
    "Blaj",
    "Borsec",
    "Botoșani",
    "Brăila",
    "Brașov",
    "București",
    "Buftea",
    "Buzău",
    "Călărași",
    "Caransebeș",
    "Cernavodă",
    "Cluj-Napoca",
    "Constanța",
    "Covasna",
    "Craiova",
    "Dej",
    "Deva",
    "Dorohoi",
    "Drobeta-Turnu Severin",
    "Drăgășani",
    "Făgăraș",
    "Fălticeni",
    "Fetești",
    "Focșani",
    "Galați",
    "Gheorgheni",
    "Giurgiu",
    "Hârșova",
    "Hunedoara",
    "Huși",
    "Iași",
    "Isaccea",
    "Lugoj",
    "Măcin",
    "Mangalia",
    "Medgidia",
    "Mediaș",
    "Miercurea Ciuc",
    "Mizil",
    "Motru",
    "Năsăud",
    "Năvodari",
    "Odobești",
    "Oltenița",
    "Onești",
    "Oradea",
    "Orșova",
    "Petroșani",
    "Piatra Neamț",
    "Pitești",
    "Ploiești",
    "Predeal",
    "Râmnicu Vâlcea",
    "Reghin",
    "Reșița",
    "Roman",
    "Roșiorii de Vede",
    "Satu Mare",
    "Sebeș",
    "Sfântu Gheorghe",
    "Sibiu",
    "Sighișoara",
    "Sinaia",
    "Slatina",
    "Slobozia",
    "Sovata",
    "Suceava",
    "Sulina",
    "Țăndărei",
    "Târgoviște",
    "Târgu Jiu",
    "Târgu Mureș",
    "Tecuci",
    "Timișoara",
    "Tulcea",
    "Turda",
    "Turnu Măgurele",
    "Urziceni",
    "Vaslui",
    "Vatra Dornei",
    "Victoria",
    "Videle",
    "Zalău",
];

pub(super) const NAME_SLOVAK_REAL: &[&str] = &[
    "Bratislava",
    "Bánovce nad Bebravou",
    "Banská Bystrica",
    "Banská Štiavnica",
    "Bardejov",
    "Brezno",
    "Brezova pod Bradlom",
    "Bytča",
    "Čadca",
    "Čierna nad Tisou",
    "Detva",
    "Prešov",
    "Dolny Kubin",
    "Spišská Nová Ves",
    "Dunajská Streda",
    "Gabčíikovo",
    "Galanta",
    "Gbely",
    "Gelnica",
    "Handlová",
    "Hlohovec",
    "Holíč",
    "Humenné",
    "Hurbanovo",
    "Kezmarok",
    "Komárno",
    "Košice",
    "Kremnica",
    "Krompachy",
    "Kuty",
    "Leopoldov",
    "Levoča",
    "Liptovský Mikuláš",
    "Lučenec",
    "Malacky",
    "Martin",
    "Medzilaborce",
    "Michalovce",
    "Modra",
    "Myjava",
    "Námestovo",
    "Nitra",
    "Nová Baňa",
    "Nové Mesto nad Váhom",
    "Nové Zámky",
    "Partizánske",
    "Pezinok",
    "Piešťany",
    "Poltár",
    "Poprad",
    "Považská Bystrica",
    "Prievidza",
    "Púchov",
    "Revúca",
    "Rimavská Sobota",
    "Rožňava",
    "Ružomberok",
    "Sabinov",
    "Šaľa",
    "Senec",
    "Senica",
    "Sereď",
    "Skalica",
    "SládkoviÍovo",
    "Smolenice",
    "Snina",
    "Stará Ľubovňa",
    "Stará Turá",
    "Strážske",
    "Stropkov",
    "Stupava",
    "Štúrovo",
    "Ġulekovo",
    "Topoľčany",
    "Trebišov",
    "Trenčín",
    "Trnava",
    "Turčianske Teplice",
    "Tvrdošín",
    "Vráble",
    "Vranov nad Topľou",
    "Záhorská Bystrica",
    "Ždiar",
    "Ľiar nad Hronom",
    "Žilina",
    "Zlaté Moravce",
    "Zvolen",
];

pub(super) const NAME_NORWEGIAN_1: &[&str] = &[
    "Arna", "Aust", "Bjørk", "Bjørn", "Brand", "Bøver", "Drag", "Drø", "Eids", "Egge", "Fager",
    "Finns", "Flat", "Foll", "Foss", "Fugle", "Furu", "Gaus", "Galte", "Geir", "Gløs", "Gran",
    "Grind", "Grims", "Grøn", "Grøt", "Gulle", "Haka", "Hammer", "Haug", "Hol", "Hon", "Hop",
    "Hov", "Jess", "Kabel", "Kjerns", "Kjerring", "Knatte", "Krok", "Køy", "Lang", "Lauv", "Leir",
    "Lund", "Logn", "Lo", "Lyng", "Løn", "Mesna", "Mel", "Mo", "Nar", "Nitte", "Nord", "Odd",
    "Ola", "Otte", "Ran", "Rev", "Rog", "Roms", "Rosen", "Sand", "Sau", "Sel", "Sol", "Sjur",
    "Skår", "Slått", "Stjør", "Stor", "Svart", "Svens", "Svin", "Sylte", "Syn", "Tran", "Vass",
    "Ved", "Vest", "Vesle", "Vik", "Våg",
];

pub(super) const NAME_NORWEGIAN_2: &[&str] = &[
    "aker", "anger", "bakken", "bekk", "berg", "botn", "breen", "bu", "bugen", "by", "bygd", "bø",
    "dal", "egga", "eid", "elv", "enga", "foss", "fjell", "fjord", "foten", "gard", "grend",
    "hammer", "haug", "havn", "heim", "hella", "hovda", "høa", "høgda", "kampen", "kjølen",
    "kollen", "kroken", "land", "lia", "mark", "moen", "myr", "nes", "nuten", "osen", "rike",
    "rud", "sand", "set", "sjøen", "skogen", "slette", "snipa", "stad", "stua", "stulen", "sund",
    "svingen", "sætra", "tinden", "tun", "vang", "vatn", "veid", "vik", "voll", "våg", "um",
    "åsen",
];

pub(super) const NAME_NORWEGIAN_REAL: &[&str] = &[
    "Alta",
    "Arendal",
    "Askim",
    "Bergen",
    "Bodø",
    "Brevik",
    "Bryne",
    "Brønnøysund",
    "Drammen",
    "Drøbak",
    "Egersund",
    "Elverum",
    "Farsund",
    "Fauske",
    "Finnsnes",
    "Flekkefjord",
    "Flora",
    "Fosnavåg",
    "Fredrikstad",
    "Førde",
    "Gjøvik",
    "Grimstad",
    "Halden",
    "Hamar",
    "Hammerfest",
    "Harstad",
    "Haugesund",
    "Holmestrand",
    "Horten",
    "Jørpeland",
    "Kirkenes",
    "Kolvereid",
    "Kongsberg",
    "Kongsvinger",
    "Kopervik",
    "Kragerø",
    "Kristiansand",
    "Kristiansund",
    "Langesund",
    "Larvik",
    "Leirvik",
    "Leknes",
    "Levanger",
    "Lillehammer",
    "Lillesand",
    "Lillestrøm",
    "Lyngdal",
    "Lørenskog",
    "Mandal",
    "Mo i Rana",
    "Molde",
    "Mosjøen",
    "Moss",
    "Mysen",
    "Måløy",
    "Namsos",
    "Narvik",
    "Notodden",
    "Odda",
    "Oslo",
    "Otta",
    "Porsgrunn",
    "Ringerike",
    "Risør",
    "Rjukan",
    "Sandefjord",
    "Sandnes",
    "Sandnessjøen",
    "Sandvika",
    "Sarpsborg",
    "Sauda",
    "Ski",
    "Skien",
    "Skudeneshavn",
    "Sortland",
    "Stathelle",
    "Stavanger",
    "Steinkjer",
    "Stjørdal",
    "Stokmarknes",
    "Stord",
    "Svelvik",
    "Svolvær",
    "Tromsø",
    "Trondheim",
    "Tvedestrand",
    "Tønsberg",
    "Ulsteinvik",
    "Vadsø",
    "Vardø",
    "Verdalsøra",
    "Åkrehamn",
    "Ålesund",
    "Åndalsnes",
];

pub(super) const NAME_HUNGARIAN_1: &[&str] = &["Nagy-", "Kis-", "Felső-", "Alsó-", "Új-"];

pub(super) const NAME_HUNGARIAN_2: &[&str] = &[
    "Bodrog",
    "Dráva",
    "Duna",
    "Hejő",
    "Hernád",
    "Rába",
    "Sajó",
    "Szamos",
    "Tisza",
    "Zala",
    "Balaton",
    "Fertő",
    "Bakony",
    "Cserhát",
    "Bihar",
    "Hajdú",
    "Jász",
    "Kun",
    "Magyar",
    "Nógrád",
    "Nyír",
    "Somogy",
    "Székely",
    "Buda",
    "Győr",
    "Pest",
    "Fehér",
    "Cserép",
    "Erdő",
    "Hegy",
    "Homok",
    "Mező",
    "Puszta",
    "Sár",
    "Császár",
    "Herceg",
    "Király",
    "Nemes",
    "Püspök",
    "Szent",
    "Almás",
    "Szilvás",
    "Agg",
    "Aranyos",
    "Békés",
    "Egyházas",
    "Gagy",
    "Heves",
    "Kapos",
    "Tápió",
    "Torna",
    "Vas",
    "Vámos",
    "Vásáros",
];

pub(super) const NAME_HUNGARIAN_3: &[&str] = &[
    "apáti", "bába", "bikk", "dob", "fa", "föld", "hegyes", "kak", "kereszt", "kürt", "ladány",
    "mérges", "szalonta", "telek", "vas", "völgy",
];

pub(super) const NAME_HUNGARIAN_4: &[&str] = &["alja", "egyháza", "háza", "úr", "vár"];

pub(super) const NAME_HUNGARIAN_REAL: &[&str] = &[
    "Ajka",
    "Aszód",
    "Badacsony",
    "Baja",
    "Budapest",
    "Debrecen",
    "Eger",
    "Fonyód",
    "Gödöllő",
    "Győr",
    "Gyula",
    "Karcag",
    "Kecskemét",
    "Keszthely",
    "Kisköre",
    "Kocsord",
    "Komárom",
    "Kőszeg",
    "Makó",
    "Mohács",
    "Miskolc",
    "Ózd",
    "Paks",
    "Pápa",
    "Pécs",
    "Polgár",
    "Sarkad",
    "Siófok",
    "Szeged",
    "Szentes",
    "Szolnok",
    "Tihany",
    "Tokaj",
    "Vác",
    "Záhony",
    "Zirc",
];

pub(super) const NAME_SWISS_REAL: &[&str] = &[
    "Aarau",
    "Aesch",
    "Altdorf",
    "Arosa",
    "Appenzell",
    "Arbon",
    "Altstätten",
    "Baar",
    "Baden",
    "Bellinzona",
    "Brig-Glis",
    "Bienne",
    "Bulle",
    "Binningen",
    "Burgdorf",
    "Bern",
    "Basel",
    "Bülach",
    "Carouge",
    "Cham",
    "Chiasso",
    "Chur",
    "Davos",
    "Delémont",
    "Dietikon",
    "Dübendorf",
    "Emmen",
    "Freienbach-Pfäffikon",
    "Fribourg",
    "Frauenfeld",
    "Genève",
    "Glarus",
    "Gossau",
    "Grenchen",
    "Herisau",
    "Horgen",
    "Horw",
    "Illnau-Effretikon",
    "Ittigen",
    "Jona",
    "Kriens",
    "Kloten",
    "Köniz",
    "Kreuzlingen",
    "Küsnacht",
    "Agen",
    "Lancy",
    "La Chaux-de-Fonds",
    "Lenzburg",
    "Lugano",
    "Langenthal",
    "Littau",
    "Le Locle",
    "La Neuveville",
    "Locarno",
    "Liestal",
    "La Tour-de-Peilz",
    "Lausanne",
    "Lyss",
    "Luzern",
    "Martigny",
    "Münchenstein",
    "Meyrin",
    "Montreux",
    "Monthey",
    "Morges",
    "Murten",
    "Moutier",
    "Muttenz",
    "Neuchâtel",
    "Neuhausen am Rheinfall",
    "Nyon",
    "Olten",
    "Onex",
    "Opfikon",
    "Ostermundigen",
    "Payerne",
    "Peseux",
    "Prilly",
    "Pully",
    "Rapperswil",
    "Richterswil",
    "Regensdorf",
    "Rheinfelden",
    "Riehen",
    "Renens",
    "Romanshorn",
    "Rorschach",
    "Stans",
    "Schaffhausen",
    "Steffisburg",
    "St. Gallen",
    "Schlieren",
    "Sierre",
    "Solothurn",
    "St. Moritz",
    "Sion",
    "Spiez",
    "Stäfa",
    "Sursee",
    "Schwyz",
    "Thalwil",
    "Thônex",
    "Thun",
    "Uster",
    "Uzwil",
    "Vernier",
    "Volketswil",
    "Versoix",
    "Vevey",
    "Wädenswil",
    "Wettingen",
    "Wil",
    "Wallisellen",
    "Winterthur",
    "Wohlen",
    "Worb",
    "Wetzikon",
    "Yverdon-les-Bains",
    "Zollikon",
    "Zofingen",
    "Zürich",
    "Zug",
];

pub(super) const NAME_DANISH_1: &[&str] = &[
    "Gamle ", "Lille ", "Nye ", "Store ", "Kirke ", "Nørre ", "Vester ", "Sønder ", "Øster ",
    "Hvide ", "Høje ", "Kongens ",
];

pub(super) const NAME_DANISH_2: &[&str] = &[
    "Ager",
    "Alle",
    "Aske",
    "Balle",
    "Bede",
    "Birke",
    "Bjerring",
    "Bjæver",
    "Blommens",
    "Blok",
    "Bolder",
    "Bred",
    "Charlotten",
    "Christians",
    "Danne",
    "Diana",
    "Es",
    "Fredens",
    "Frederiks",
    "Fugle",
    "Fåre",
    "Gille",
    "Gis",
    "Givs",
    "Glams",
    "Glo",
    "Guld",
    "Had",
    "Haralds",
    "Hassel",
    "Hede",
    "Helle",
    "Hessel",
    "Hjorts",
    "Hol",
    "Horn",
    "Humle",
    "Høj",
    "Hør",
    "Is",
    "Jyde",
    "Jægers",
    "Karls",
    "Klov",
    "Kokke",
    "Kvist",
    "Lang",
    "Lange",
    "Mari",
    "Nord",
    "Ny",
    "Oks",
    "Ring",
    "Røde",
    "Rung",
    "Rør",
    "Rud",
    "Saks",
    "Salt",
    "Skam",
    "Silke",
    "Skod",
    "Skæl",
    "Skær",
    "Sol",
    "Svend",
    "Svine",
    "Strand",
    "Stubbe",
    "Ting",
    "Tjære",
    "Tore",
    "Uger",
    "Ulf",
    "Val",
    "Vand",
    "Vej",
    "Vor",
    "Vær",
    "Ør",
    "Ål",
];

pub(super) const NAME_DANISH_3: &[&str] = &[
    "basse", "borg", "berg", "bro", "by", "havn", "strup", "holm", "hus", "købing", "lund",
    "lunde", "sund", "ovre", "høj", "dal", "sted", "sten", "løse", "rød", "magle", "sø", "bjerg",
    "bæk", "drup", "lev", "bo", "lyst", "feld", "skov",
];

pub(super) const NAME_TURKISH_PREFIX: &[&str] = &[
    "Akça", "Altın", "Bahçe", "Boz", "Büyük", "Çay", "Doğu", "Eski", "Güzel", "Kızıl", "Küçük",
    "Orta", "Sarı", "Sultan", "Ulu", "Yeni",
];

pub(super) const NAME_TURKISH_MIDDLE: &[&str] =
    &["agaç", "ayva", "çam", "elma", "kurt", "pazar", "yalı"];

pub(super) const NAME_TURKISH_SUFFIX: &[&str] = &[
    "dere", "hisar", "kale", "kaya", "kent", "köy", "ova", "özü", "ören", "pazar", "saray", "tepe",
    "yer", "yurt",
];

pub(super) const NAME_TURKISH_REAL: &[&str] = &[
    "Adana",
    "Adıyaman",
    "Afyon",
    "Ağrı",
    "Amasya",
    "Antalya",
    "Artvin",
    "Balıkesir",
    "Bilecik",
    "Bitlis",
    "Bolu",
    "Burdur",
    "Bursa",
    "Çanakkale",
    "Çankırı",
    "Denizli",
    "Diyarbakır",
    "Edirne",
    "Elazığ",
    "Erzurum",
    "Eskişehir",
    "Giresun",
    "Gümüşhane",
    "Hatay",
    "Isparta",
    "İçel",
    "İstanbul",
    "İzmir",
    "Kars",
    "Kastamonu",
    "Kayseri",
    "Kirklareli",
    "Kocaeli",
    "Konya",
    "Kütahya",
    "Malatya",
    "Manisa",
    "Kahramanmaraş",
    "Mardin",
    "Muğla",
    "Muş",
    "Nevşehir",
    "Niğde",
    "Rize",
    "Sakarya",
    "Samsun",
    "Siirt",
    "Sinop",
    "Sivas",
    "Trabzon",
    "Şanlıurfa",
    "Van",
    "Yozgat",
    "Zonguldak",
    "Aksaray",
    "Bayburt",
    "Karaman",
    "Şırnak",
    "Bartın",
    "Ardahan",
    "Iğdır",
    "Yalova",
    "Karabük",
    "Osmaniye",
    "Düzce",
];

pub(super) const NAME_ITALIAN_REAL: &[&str] = &[
    "Roma",
    "Milano",
    "Napoli",
    "Torino",
    "Venezia",
    "Firenze",
    "Palermo",
    "Genova",
    "Parma",
    "Bologna",
    "Bari",
    "Cagliari",
    "Sassari",
    "Pisa",
    "Aosta",
    "Brescia",
    "Verona",
    "Bolzano",
    "Padova",
    "Udine",
    "Trieste",
    "Livorno",
    "Ancona",
    "Perugia",
    "Pescara",
    "L'Aquila",
    "Campobasso",
    "Potenza",
    "Cosenza",
    "Reggio Calabria",
    "Catania",
    "Caltanisetta",
    "Agrigento",
    "La Spezia",
    "Modena",
    "Vicenza",
    "Mantova",
    "Cremona",
    "Piacenza",
    "Reggio Emilia",
    "Foggia",
    "Benevento",
    "Salerno",
    "Catanzaro",
    "Lecce",
    "Como",
    "Lecco",
    "Sondrio",
    "Trento",
    "Desenzano",
    "Cuneo",
    "Asti",
    "Lodi",
    "Novara",
    "Biella",
    "Vercelli",
    "Rieti",
    "Nuoro",
    "Oristano",
    "Matera",
    "Taranto",
    "Varese",
    "Bergamo",
    "Pavia",
    "Caserta",
    "Frosinone",
    "Latina",
    "Enna",
    "Ragusa",
    "Siracusa",
    "Pordenone",
    "Imperia",
    "Verbania",
    "Alessandria",
    "Messina",
    "Siena",
    "Arezzo",
    "Grosseto",
];

pub(super) const NAME_ITALIAN_PREF: &[&str] = &[
    "Alpe ", "Borgo ", "Cascina ", "Castel ", "Fonte ", "Forte ", "Malga ", "Pieve ", "Poggio ",
    "Rocca ", "Villa ", "Villar ",
];

pub(super) const NAME_ITALIAN_1M: &[&str] = &[
    "Bel", "Borgo", "Bosco", "Campo", "Capo", "Casal", "Castel", "Colle", "Fiume", "Fonte", "Lago",
    "Mezzo", "Monte", "Mon", "Orto", "Passo", "Prato", "Poggio", "Ponte", "Pozzo", "Sasso", "Tra",
    "Tre", "Ver", "Vico",
];

pub(super) const NAME_ITALIAN_1F: &[&str] = &[
    "Acqua", "Bra", "Cala", "Casa", "Chiesa", "Citta", "Civita", "Corte", "Costa", "Croce",
    "Fontana", "Grotta", "Guardia", "Mezza", "Palma", "Pietra", "Ripa", "Rocca", "Serra", "Torre",
    "Val", "Valle", "Villa",
];

pub(super) const NAME_ITALIAN_2: &[&str] = &[
    "bell", "bianc", "cald", "chiar", "cort", "ferrat", "fier", "fredd", "gioios", "grec", "guzz",
    "lung", "long", "migli", "negr", "ner", "nov", "nuov", "ross", "rotond", "scur", "secc",
    "sett", "vecchi", "ventos", "vers", "viv",
];

pub(super) const NAME_ITALIAN_2I: &[&str] = &[
    "", "breve", "brevi", "chiari", "ferro", "fieschi", "fiore", "fonte", "forte", "gate", "leone",
    "maggiore", "minore", "mole", "monte", "poli", "scuri", "terra", "te", "torrione", "vento",
    "verde", "versiere",
];

pub(super) const NAME_ITALIAN_3: &[&str] = &[
    " Marittimo",
    " Marittima",
    " del Capo",
    " del Monte",
    " di Sopra",
    " di Sotto",
    " sui Monti",
    " dei Marmi",
    " dei Sassi",
    " delle Fonti",
    " sui Prati",
    " a Mare",
    " Superiore",
    " Inferiore",
    " Terme",
    " Alta",
    " Bassa",
    " Brianza",
    " Vesuviano",
    " Scrivia",
    " Ticino",
];

pub(super) const NAME_ITALIAN_RIVER1: &[&str] = &[" del", " sul", " al", " nel"];

pub(super) const NAME_ITALIAN_RIVER2: &[&str] = &[
    "l'Adda",
    "l'Adige",
    "le Alpi",
    "l'Arno",
    " Bormida",
    " Brenta",
    "la Dora Baltea",
    " Lambro",
    " Mincio",
    " Naviglio",
    "l'Oglio",
    "l'Olona",
    "l'Ombrone",
    " Panaro",
    " Piave",
    " Po",
    " Reno",
    " Scrivia",
    " Secchia",
    " Serio",
    " Tagliamento",
    " Tanaro",
    " Taro",
    " Ticino",
    " Tevere",
];

pub(super) const NAME_CATALAN_REAL: &[&str] = &[
    "Barcelona",
    "L'Hospitalet de Llobregat",
    "Cerdanyola",
    "Martorell",
    "Badalona",
    "Tarragona",
    "Lleida",
    "Girona",
    "Sabadell",
    "Terrassa",
    "Reus",
    "Valls",
    "Vic",
    "Vielha e Mijaran",
    "Amposta",
    "Tortosa",
    "Berga",
    "Olot",
    "Mollerussa",
    "Banyoles",
    "Figueres",
    "Balaguer",
    "Vilafranca del Penedès",
    "La Seu d'Urgell",
    "El Pont de Suert",
    "Igualada",
    "Manresa",
    "Solsona",
    "Les Borges Blanques",
    "Tremp",
    "Sort",
    "Colera",
    "Portbou",
    "El Vendrell",
    "Falset",
    "Ripoll",
    "Cervera",
    "Gandesa",
    "Mataró",
    "Montblanc",
    "Vilanova i la Geltrú",
    "Tàrrega",
    "Camprodon",
    "Campdevànol",
    "Cambrils",
    "Begur",
    "Setcases",
    "Palafrugell",
    "Begues",
    "El Bruc",
    "Cadaqués",
    "Collbató",
    "Cervelló",
    "Esparreguera",
    "Abrera",
    "Alp",
    "Das",
    "Cercs",
    "Manlleu",
    "El Masnou",
    "Molins de Rei",
    "Monistrol",
    "Rocallaura",
    "Rubí",
    "Ripollet",
    "Sitges",
    "Roses",
];

pub(super) const NAME_CATALAN_PREF: &[&str] = &[
    "El Pont de ",
    "Parets de ",
    "Canet de ",
    "Castellar de ",
    "Corbera de ",
    "Arenys de ",
    "Calella de ",
    "La Seu de ",
    "La Bisbal de ",
    "Torroella de ",
    "Port de ",
    "Vilafranca de ",
    "Vilanova de ",
    "Caldes de ",
    "La Conca de ",
    "Olesa de ",
    "La Roca de ",
    "Sant Esteve de ",
    "Sant Andreu de ",
    "Sant Jordi de ",
    "Sant Joan de ",
    "Sant Feliu de ",
    "Sant Quirze de ",
    "Sant Sadurní de ",
    "Santa Coloma de ",
    "Santa Margarida de ",
    "Santa Maria de ",
    "Sant Martí de ",
    "Sant Pere de ",
    "Sant Julià de ",
    "Sant Vicenç de ",
];

pub(super) const NAME_CATALAN_1M: &[&str] = &["Torrent", "Cami", "Mont", "Bell", "Puig", "Riu"];

pub(super) const NAME_CATALAN_1F: &[&str] = &[
    "Pala", "Selva", "Vall", "Serra", "Torre", "Riba", "Cova", "Terra",
];

pub(super) const NAME_CATALAN_2M: &[&str] = &[
    "alt", "baix", "fosc", "pelat", "ventós", "negre", "roig", "grís",
];

pub(super) const NAME_CATALAN_2F: &[&str] = &[
    "baixa", "alta", "fosca", "clara", "negra", "roja", "grisa", "freda",
];

pub(super) const NAME_CATALAN_3: &[&str] = &[
    " Despí",
    " Desvern",
    " del Camí",
    " de Mar",
    " de Dalt",
    " de Baix",
    " del Vallès",
    " de Berguedà",
    " de Conflent",
    " de la Plana",
];

pub(super) const NAME_CATALAN_RIVER1: &[&str] = &[
    " d'Anoia",
    " de Ter",
    " de Llobregat",
    " d'Ebre",
    " de Segre",
    " de Francolí",
];

/// Singular/plural grammatical genders from `table/townname.h`.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct CzechGender(pub u8);

pub(super) const CZG_SMASC: CzechGender = CzechGender(0);
pub(super) const CZG_SFEM: CzechGender = CzechGender(1);
pub(super) const CZG_SNEUT: CzechGender = CzechGender(2);
pub(super) const CZG_PMASC: CzechGender = CzechGender(3);
pub(super) const CZG_PFEM: CzechGender = CzechGender(4);
pub(super) const CZG_PNEUT: CzechGender = CzechGender(5);
/// Special for substantive stems: the ending chooses the gender.
pub(super) const CZG_FREE: CzechGender = CzechGender(6);
/// Like `CZG_FREE`, but disallow `CZG_SNEUT`.
pub(super) const CZG_NFREE: CzechGender = CzechGender(7);

/// Adjective declension pattern from `table/townname.h`.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub(super) struct CzechPattern(pub u8);

pub(super) const CZP_JARNI: CzechPattern = CzechPattern(0);
pub(super) const CZP_MLADY: CzechPattern = CzechPattern(1);
pub(super) const CZP_PRIVL: CzechPattern = CzechPattern(2);

/// A substantive can choose only some adjectives/endings.
/// At least one of these flags must be satisfied.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub(super) struct CzechAllowFlag(pub u8);

#[allow(non_upper_case_globals)]
impl CzechAllowFlag {
    pub const Short: Self = Self(0);
    pub const Middle: Self = Self(1);
    pub const Long: Self = Self(2);
}

/// Native `EnumBitSet<CzechAllowFlag, uint8_t>`; elements are bit positions.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub(super) struct CzechAllowFlags(pub u8);

impl CzechAllowFlags {
    pub const fn from_element(element: CzechAllowFlag) -> Self {
        Self(1 << element.0)
    }
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

pub(super) const CZA_ALL: CzechAllowFlags = CzechAllowFlags::from_element(CzechAllowFlag::Short)
    .with(CzechAllowFlags::from_element(CzechAllowFlag::Middle))
    .with(CzechAllowFlags::from_element(CzechAllowFlag::Long));

/// All these flags must be satisfied, in the stem-to-others direction.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub(super) struct CzechChooseFlag(pub u8);

#[allow(non_upper_case_globals)]
impl CzechChooseFlag {
    pub const Colour: Self = Self(0);
    /// Matched if a postfix was inserted.
    pub const Postfix: Self = Self(1);
    /// Matched if no postfix was inserted.
    pub const NoPostfix: Self = Self(2);
}

/// Native `EnumBitSet<CzechChooseFlag, uint8_t>`; elements are bit positions.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub(super) struct CzechChooseFlags(pub u8);

impl CzechChooseFlags {
    pub const EMPTY: Self = Self(0);
    pub const fn from_element(element: CzechChooseFlag) -> Self {
        Self(1 << element.0)
    }
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    pub const fn test(self, element: CzechChooseFlag) -> bool {
        self.0 & Self::from_element(element).0 != 0
    }
    pub const fn all(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

pub(super) const CZC_ANY: CzechChooseFlags =
    CzechChooseFlags::from_element(CzechChooseFlag::Colour)
        .with(CzechChooseFlags::from_element(CzechChooseFlag::Postfix))
        .with(CzechChooseFlags::from_element(CzechChooseFlag::NoPostfix));

#[derive(Clone, Copy)]
pub(super) struct CzechNameSubst {
    pub gender: CzechGender,
    pub allow: CzechAllowFlags,
    pub choose: CzechChooseFlags,
    pub name: &'static str,
}
#[derive(Clone, Copy)]
pub(super) struct CzechNameAdj {
    pub pattern: CzechPattern,
    pub choose: CzechChooseFlags,
    pub name: &'static str,
}
