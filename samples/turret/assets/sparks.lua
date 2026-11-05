-- Sparks, the drone's third component, written in Lua: it reacts to the
-- armour denting (Rust, in armour.rs) and to the drone being destroyed
-- (Health, in main.lst).

local Sparks = component("Sparks", {
    { "shown", "int", 0 },
})

function Sparks:on_Dented(platesLeft)
    self.shown = self.shown + 1
    solis.World:say("Sparks fly from " .. solis.entity.name(self.me) .. ": " .. platesLeft .. " armour plates left.")
end

function Sparks:on_Destroyed()
    solis.World:say(solis.entity.name(self.me) .. " is destroyed!")
    -- It drops to the ground.
    local at = solis.entity.position(self.me)
    solis.entity.moveTo(self.me, vec3(at.x, at.y, 0))
end
