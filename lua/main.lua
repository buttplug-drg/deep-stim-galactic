local lb = require("luabutt")
local uhelpers = require("UEHelpers")

local function printf(s, ...)
    return print("[deepcock] " .. string.format(s, ...))
end

lb.init(12345)


local last_location = nil
local function log_player_location()
    -- local player_controller = uhelpers:GetPlayerController()
    -- local player_pawn = player_controller.pawn
    local player_character = uhelpers:GetPlayer()
    local location = player_character:K2_GetActorLocation()
    print(string.format("Player location: {X=%.3f, Y=%.3f, Z=%.3f}\n", location.X, location.Y, location.Z))
    if last_location then
        printf("Player moved: {delta_X=%.3f, delta_Y=%.3f, delta_Z=%.3f}\n",
               location.X - last_location.X,
               location.Y - last_location.Y,
               location.Z - last_location.Z)
    end
    last_location = location
end

local function get_player_health_component()
    return uhelpers:GetPlayer().HealthComponent
end

local function get_player_health()
    local player_health_component = get_player_health_component()

    -- local player_character = uhelpers:GetPlayer()
    -- local player_health_component = player_character.HealthComponent
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

    return player_health_component:GetHealth()
end

local function damage_player(amount)
    local health_component = get_player_health_component()
    health_component:TakeDamageSimple(amount, nil, nil)
end

local function nop() end

local function register_keybinds()
    RegisterKeyBind(Key.F1, function()
        printf("hit F1")
        log_player_location()
    end)
    RegisterKeyBind(Key.F2, function()
        printf("hit F2")
        printf("%f", get_player_health())
    end)
    local next_val = 0.5
    RegisterKeyBind(Key.F3, function()
        printf("hit F3")
        lb.set_vibration(next_val)
        if next_val == 0 then
            next_val = 0.5
        else
            next_val = 0
        end
    end)
    RegisterKeyBind(Key.F4, function()
        printf("hit f4")
        damage_player(10)
    end)

    -- Some quick testing reveals that the function to hook to do things at the start of the round is called
    --  /Script/FSD.FSDGameMode:StartGame
    -- TODO: figure out if multiplayer is special
    -- TODO: figure out if deepdives are special

    RegisterHook("/Script/FSD.FSDGameMode:StartGame",
        function()
            print("Function /Script/FSD.FSDGameMode:StartGame start")
        end,
        function()
            print("Function /Script/FSD.FSDGameMode:StartGame end")
        end)
    -- RegisterHook("/Script/Engine.GameMode:RestartGame",
    --     function()
    --         print("Function /Script/Engine.GameMode:RestartGame start")
    --     end,
    --     function()
    --         print("Function /Script/Engine.GameMode:RestartGame end")
    --     end)
    -- RegisterHook("/Game/Game/BP_GameState.BP_GameState_C:StartGame",
    --     function()
    --         print("Function /Game/Game/BP_GameState.BP_GameState_C:StartGame happened")
    --     end)

    RegisterKeyBind(Key.F5, function()
    end)
end
RegisterKeyBind(Key.F10, function()
    printf("soft-reloading.")
    register_keybinds()
end)
