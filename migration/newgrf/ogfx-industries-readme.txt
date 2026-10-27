OpenGFX+ Industries
-------------------

This version: OpenGFX+ Industries 0.3.5

Contents:

1 About
2 Compatibility
3 Features
4 Development
5 License
6 Credits



-------
1 About
-------

OpenGFX+ Industries slightly extends the existing OpenTTD industries and
fixes issues like certain industries never closing so that one will not
end up with tons of those industries on the map.


Name of this Repo:  OpenGFX+ Industries 0.3.5
Repository version: 5199
GRF_ID:
MD5 sum:            {{GRF_MD5}}



---------------
2 Compatibility
---------------

OpenGFX is basically incompatible with all known industry sets as it
re-defines most industries; thus it will de-activate itself when the
presence of another known industry NewGRF is detected.


2. 1 Total Town Replacement Set
-------------------------------

The Total Town Replacement Set defines the banks as industries as well
and has no option to actually disable them. Thus OpenGFX+ Industries
takes this into account and disables its own modification of the banking
sector. None of the changes to the bank chain and its related cargo
availability (valuables, gold, diamonds) will take effect.



----------------------
3 Features
----------------------

OpenGFX+Industries allows to customize the industry chains independent
of the climate and to select the accepted and output cargos of a range
of industries according to your desires.

3.1 Parameters
--------------

* Oil refinery distance:
    Allows to set the maximum distance for oil refineries from the map
    border. Alternatively allows to disable this restriction completely.
* Oil wells restrictions:
    Allows oil wells to increase production and to disable the building
    restrictions for oil wells, including their limitation to years
    earlier than 1950.
* Oil rigs building restrictions:
    Allows to build oil rigs prior to 1960 and during map generation
* Valuables bank restriction:
    Allows banks which accept valuables to be built also during map
    generation
* Coal chain:
    Configures the coal chain
    Producer:   coal mine (Coal)
    Recipient:  powerplant (Coal)
* Oil chain:
    Configures the oil chain
    Producer:   oil well, oil rig (Oil)
    Recipient:  oil refinery (Oil)
* Valuables chain:
    Configures the valuable chain to valuables, diamonds and / or gold.
    Producer:   Diamond mine (Diamonds)
                Gold mine    (Gold)
                Bank         (Valuables)
    Recipients: Bank         (Diamonds, Gold, Valuables)
* Iron ore chain:
    Configures the iron ore chain
    Producer:   Iron ore mine
    Recipient:  Steel mill
*Copper ore chain:
    Configures the copper ore chain
    Producer:   Copper ore mine
    Recipient:  Factory
* Rubber chain:
    Configures the rubber chain
    Producer:   Rubber plantation
    Recipient:  Factory
* Farm chain:
    Configures the farm chain
    Producer:   Farm (Grain and / or livestock)
* Farm destination:
    Configures the destination of farm cargos
    Recipient:  Factory or Food Processing Plant
    Note that depending on the presence of other industry chains the
    Food Processing Plant might be chosen whatever this setting
* Fruit chain:
    Configure the presence of the fruit plantation
    Producer:   Fruit plantation (fruits)
    Recipient:  Shops
* Wood chain:
    Configure the wood source
    Producer: forest or lumber mill
* Wood destination:
    Configure the destination for wood
    Recipient:  Sawmill, paper mill or factory
    Note that depending on the presence of other industry chains the
    factory destination might be ignored and sawmill chosen instead
* Water chain:
    Configure the presence of the water chain
    Producer:   Water well
    Recipient:  Water tower



----------------------
4 Development
----------------------

Requirements for building this NewGRF successfully:
	NML (0.3 or nightly)
	gcc
	awk
	grep
	md5sum (or md5 on Mac)
	make
    mercurial (recommended)
    python (recommended)
If you want to bundle the grf, you'll need additionally
	tar
	zip
	bzip2
	unix2dos (optional)

The source code can be obtained from the #openttdcoop DevZone at
http://dev.openttdcoop.org/projects/ogfx-industries
or via mercurial checkout
hg clone http://hg.openttdcoop.org/ogfx-industries



---------
5 License
---------

This NewGRF was written by the authors as listed in the credits section
and is free to use for anyone under the terms of the GNU Pulic License
version 2 or higher. See license.txt.



---------
6 Credits
---------

Authors: Ingo von Borstel (aka planetmaker)
         José Ángel Soler Ortiz (aka Terkhen)
         Industry map colours:      3iff

Graphics:
	Snowy oil well ground tile: 2006TTD
	Gold mine sprites:          Zephyris (modified by planetmaker)
	Diamond mine sprites:       Zephyris (modified by planetmaker)
	Iron ore mine sprites:      Zephyris (modified by planetmaker)
	Shop sprites:               Irwe     (modified by andythenorth)

Translations:
	Catalan:                    Juanjo, Bassals
	Dutch:                      Alberth
	Finnish:                    juzza1
	German:                     planetmaker, Jogio
	Greek:                      Tafidis
	Hungarian:                  molace
	Indonesian:                 UseYourIllusion, Yoursnotmine
	Korean:                     Telk
	Kroatian:                   Voyager1
	Norwegian (bokmal)          Trond
	Russian:                    Deicide, George
	Scotish Gaelic:             GunChloeoc
	Spanish:                    Terkhen, SilverSurferZzZ
