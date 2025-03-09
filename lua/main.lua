local lb = require("luabutt")
local uhelpers = require("UEHelpers")

local function printf(s, ...)
    return print("[deepcock] " .. string.format(s, ...))
end

printf("Hewwo!!")
lb.hello_from_rs()
lb.init(12345)

local last_location = nil
local function log_player_location()
    local player_controller = uhelpers:GetPlayerController()
    local player_pawn = player_controller.pawn
    local location = player_pawn:K2_GetActorLocation()
    print(string.format("Player location: {X=%.3f, Y=%.3f, Z=%.3f}\n", location.X, location.Y, location.Z))
    if last_location then
        printf("Player moved: {delta_X=%.3f, delta_Y=%.3f, delta_Z=%.3f}\n",
               location.X - last_location.X,
               location.Y - last_location.Y,
               location.Z - last_location.Z)
    end
end

local function log_player_health()
    local player_controller = uhelpers:GetPlayerController()
    local player_pawn = player_controller.pawn

    local player_health_component = player_pawn.HealthComponent
    -- this is why i hate OO systems.
    -- it's not that OO is inherently terrible. it's just that the ppl who design this sorta shit
    -- tend to get sooo on their asses about "the interface" and "oooo must stay SOLID"
    -- FUCK YOU
    -- i tried for so goddamn long to get this working in the following way:
    --[[
        local player_health_component = player_pawn:GetHealthComponent()
    --]]
    -- and it just fails
    -- why? fuck you
    -- it fails.
    -- and i think "oh surely theres a reason this isnt directly accessible"
    -- "surely maybe it has to be a private member for whichever reaso-"
    -- IT ISNT EVEN A PRIVATE MEMVBER
    -- as far as i can tell, the only reason that method exists is because blueprints cant handle
    -- just accessing a fucking property on a class instance
    -- so theres this stupid ass wrapper function with no obvious return type *that shows up in the
    -- debugger as a fucking object*
    -- and it's just useless to me.
    -- great fucking red herring there, Ghost Ship.
    -- fuck you.
    -- and while we're at it, Dear Unreal Engine devs, what the fuck is a FloatProperty,
    -- and why does it come from   s e v e n   layers of inheritance????????
    -- fuck you fuck you fuck you

    printf("Player health: %f", player_health_component:GetHealth())
end

RegisterKeyBind(Key.F1, function()
    printf("hit F1")
    log_player_location()
end)
RegisterKeyBind(Key.F2, function()
    printf("hit F2")
    log_player_health()
end)
