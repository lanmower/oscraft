# Combat, transport, cloak and burrow fields. unit_extra(name, ...) sets them for one unit.
unit_extra("skirmisher", size="small", damage_type="normal")
unit_extra("warden", size="large", damage_type="explosive", transport_capacity=8, ranged_miss=6554, rally_dx=1000, rally_dy=0)
unit_extra("tender", size="medium")
unit_extra("spore", size="small", cloak=["burrowable"], burrow_ticks=24)
