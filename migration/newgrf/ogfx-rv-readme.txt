OpenGFX+ Road Vehicles 0.4.1
-----------------------------------

Contents:

1 About
2 Details
  2.1 Parameters
  2.2 Available vehicles
3 License
4 Credits



-------
1 About
-------

OpenGFX+ Road Vehicles enhances default road vehicles while keeping a design
that matches the OpenGFX style.

 • Cargo support:
    Provides a set of vehicles that are able to carry all NewGRF cargos known at
    the moment of release. Many of these cargos have an unique graphical
    representation.

 • Trams:
    A few tramcars appear in OpenGFX+ Road Vehicles. They are able to carry
    passengers and tourists.

 • Auto-Refit:
    Trucks and buses can be refitted between different cargos appropiate for
    their type. Depending on the change they can even be refitted free-of-charge
    in a road stop or at a cost in a depot.

 • Adjustable costs:
    Road vehicle purchase and running costs can be set by the user. See section
    2.1 for details.

 • Refrigerated trucks:
    Cargo ages more slowly in refrigerated trucks, which makes them better for
    moving perishable products over large distances.


Name of this Repo:  OpenGFX+ Road Vehicles 0.4.1
Repository version: 5405
GRF_ID:
MD5 sum:            {{GRF_MD5}}



--------------
2 Details
--------------

The minimum required OpenTTD version for this NewGRF is:

 • Stable: OpenTTD 1.2.0

 • Trunk:  OpenTTD r23670



2.1 Parameters:
---------------

Choice of vehicles:
    These parameters can be used to decide which types of vehicles from OpenGFX+
    Road Vehicle should appear in the game. It is possible to enable and/or
    disable buses, trucks and trams separately.


Purchase and running costs:
    It is possible to increase or decrease purchase and running costs manually
    by modifying the appropiate parameters for each cost.



2.2 Available vehicles:
-----------------------

The number of available trucks and buses has been reduced in comparison with the
default vehicles, but each model can be refitted to different cargos. All cargos
will have at least a vehicle that can carry them, and in some cases different
trucks can carry the same cargo. Check their capacities and costs to decide
which one is the right model for your route.


Buses:
 • Can be refitted for free at stops.

 • Can carry passengers and tourists.


Mail trucks:
 • Can carry mail.


Armoured trucks:
 • Can be refitted for free at stops.

 • Can carry mail, gold, diamonds, and valuables.


Bulk trucks:
 • Can be refitted to cargos in the same group at stops and depots.

 • Can be refitted to cargos in different groups at depots at normal cost.

 • Dirty: Bauxite, cement, clay, coal, copper ore, fertiliser, gravel, iron ore,
   limestone, lumber, recyclables, rubber, sand, scrap metal and sulphur. Refits
   in stops are free.

 • Clean: Cereal, cotton candy, fibre crops, fruit, fruit and vegetables, grain,
   maize, oil seeds, sugar, sugar beet, sugar cane, toffee and wheat. Refits in
   stops cost half.


Flatbed trucks:
 • Can be refitted to cargos in the same group at stops and depots for free.

 • Can be refitted to cargos in different groups at depots at normal cost.

 • Vehicles: Engineering supplies, farm supplies and vehicles.

 • Containers: Building materials, bricks, dyes, fertiliser, glass, goods,
   manufacturing supplies, sweets.

 • Stakes: Batteries, bubbles, copper, fizzy drinks, lumber, metal, paper,
   petrol, plastic, refined products, steel, and wood.


Livestock trucks:
 • Can carry livestock.


Piece goods trucks:
 • Can be refitted for free at stops and depots.

 • Can carry alcohol, batteries, copper, dyes, engineering supplies, farm
   supplies, fizzy drinks, food, fruit, fruit and vegetables, goods, lumber,
   manufacturing supplies, paper, plastic, sweets, toys and wool.


Refrigerator trucks:
 • Cargo carried by this vehicle ages more slowly, and therefore is paid better
   after long trips.

 • Can be refitted for half of the cost at stops and depots.

 • Can carry fish, food, fruits, fruits and vegetables and milk.


Tank trucks:
 • Can be refitted to cargos in the same group at stops and depots.

 • Can be refitted to cargos in different groups at depots at normal cost.

 • Dirty: Cement, chemicals, fuel oil, oil, plastic, refined products, rubber.
   Refits in stops have normal cost.

 • Clean: Alcohol, Cola, dyes, fizzy drinks, milk, water. Refits in stops cost
   half. Can be refitted in stops to dirty cargos by a quarter of the normal
   cost.


---------
3 License
---------

OpenGFX+ Road Vehicles NewGRF for OpenTTD and TTDPatch
Copyright (C) 2010-2011 by the OpenGFX+ Road Vehicles team (see below)

This program is free software; you can redistribute it and/or modify it
under the terms of the GNU General Public License version 2 (or, at your
discretion, any later version) as published by the Free Software Foundation.

This program is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License
for more details.

You should have received a copy of the GNU General Public License along
with this program; if not, write to the
Free Software Foundation, Inc.
1 Franklin Street, Fifth Floor
Boston, MA 02110-1301
USA.

The source code can be obtained from the #openttdcoop DevZone at:
http://dev.openttdcoop.org/projects/ogfx-rv

Alternatively, you can get it via mercurial checkout:
hg clone http://hg.openttdcoop.org/ogfx-rv



---------
4 Credits
---------

Authors:
    Lead programmer:              José Ángel Soler Ortiz (aka Terkhen)
    General coding:               Ingo von Borstel (aka planetmaker)

8bpp Graphics:
	Trams:                        DanMacK
	Bulk trucks:                  Zephyris
	Default bulk truck cargos:    Zephyris
	Scrap metal for bulk truck:   molace
	Other bulk cargos:            Terkhen
	Flatbed trucks:               Zephyris
	Default flatbed truck cargos: Zephyris
	Other flatbed cargos:         molace
	Piece goods trucks:           Zephyris
	Refrigerated trucks:          Zephyris
	Tank trucks:                  Zephyris
	Toyland buses:                DanMack
	Toyland trucks:               DanMack

Translations:
	Afrikaans:                    te_lanus
	Catalan:                      juanjo
	Chinese (traditional):        siu238X
	Croatian:                     Voyager1
	Dutch:                        Foobar, Alberth
	Finnish:                      Arexander, juzza1
	French:                       OliTTD
	German:                       planetmaker
	Hungarian:                    molace, zaza
	Indonesian:                   UseYourIllusion
	Korean:                       kevin
	Lithuanian:                   RunisLabs
	Luxembourgish:                Phreeze
	Hungarian:                    DXTR
	Norwegian (bokmal):           Hafting, Trond
	Russian:                      akasoft, George
	Scotish Gaelic:               GunChleoc
	Spanish:                      Terkhen


Special thanks to #openttdcoop and especially Ammler who provides and works a
lot on maintaining the Development Zone where this repository is hosted and who
also frequently gives much valuable input. Thanks also to planetmaker, Alberth,
Yexo, Rubidium and Ammler who frequently give valuable input in form of advice
and patches to this project. Last but not least thanks to all the NewGRF authors
whose NewGRFs can be my playground for this project.
