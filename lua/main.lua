local luabutt = require("luabutt")
local uhelpers = require("UEHelpers")

print("Hello from Lua!")
luabutt.hello_from_rs()

local last_location = nil
function read_player_location()
    local player_controller = uhelpers:GetPlayerController()
    local player_pawn = player_controller.pawn
    local location = player_pawn:K2_GetActorLocation()
    print(string.format("[deepcock] Player location: {X=%.3f, Y=%.3f, Z=%.3f}\n", location.X, location.Y, location.Z))
    if last_location then
        print(string.format("[deepcock] Player moved: {delta_X=%.3f, delta_Y=%.3f, delta_Z=%.3f}\n",
                            location.X - last_location.X,
                            location.Y - last_location.Y,
                            location.Z - last_location.Z))
    end
end

uhelpers:RegisterKeybind(uhelpers.Key.F1, function()
    print("hit F1")
    read_player_location()
end)
