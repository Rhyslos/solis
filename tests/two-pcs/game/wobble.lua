-- Lua's share of the two-PC check's game: a lantern that bobs and sways
-- with a little trigonometry, and counts the drones' dents it hears of.

local Wobble = component("Wobble", {
    { "t", "float", 0 },
    { "height", "float", 0 },
    { "dents", "int", 0 },
})

function Wobble:update(dt)
    self.t = self.t + dt
    local t = self.t
    self.height = 1.5 + 0.5 * math.sin(t * 2.3) * math.cos(t * 0.7) + math.sqrt(t) * 0.05
    local x = math.exp(-t * 0.1) * math.cos(t)
    local y = math.log(1 + t) * 0.3 + math.atan2(t, 3)
    solis.entity.moveTo(self.me, vec3(-4 + x, 4 + y, self.height))
end

function Wobble:on_Dented(platesLeft)
    self.dents = self.dents + 1
end
