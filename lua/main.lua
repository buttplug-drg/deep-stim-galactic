local function printf(s, ...)
    return print("[deepcock] " .. string.format(s, ...))
end

printf("attempt to load luabutt")
-- TODO: uncomment
local lb = require("luabutt")
printf("luabutt successfully loaded")
local uhelpers = require("UEHelpers")

-- TODO: uncomment
-- printf("init buttplug thing..")
-- lb.init(12345)
-- printf("init buttplug thing complete uwu")

-- the player character
    -- on targed damaged: PlayerCharacter:Client_TargetDamaged(...)
    -- on weapon fire start: PlayerCharacter:OnFirePressed
    -- on weapon fire stop: PlayerCharacter:OnFireReleased

local function get_player_health_component()
    return uhelpers:GetPlayer().HealthComponent
end

local function get_player_health()
    local player_health_component = get_player_health_component()

    printf("health component names:")
    printf("fname: %s", player_health_component.OnPlayerHit:GetFName())
    printf("fullname: %s", player_health_component.OnPlayerHit:GetFullName())
    return player_health_component:GetHealth()
end

local function damage_player(amount)
    local health_component = get_player_health_component()
    health_component:TakeDamageSimple(amount, nil, nil)
end

local function nop() end

local last_shield_damage
local last_health_damage
local last_shield_damage_time = os.clock()
local last_health_damage_time = os.clock()



ExecuteInGameThread(function()
    LoadAsset("/Game/Character/BP_PlayerCharacter.BP_PlayerCharacter_C")
    -- LoadAsset must be exec'd from game thread, and the hooks must wait for the asset to be loaded.
    -- since LoadAsset is blocking, (but ExecuteInGameThread isnt), it's easiest to just do that in
    -- the game thread too
    RegisterHook("/Game/Character/BP_PlayerCharacter.BP_PlayerCharacter_C:BndEvt__HealthComponent_K2Node_ComponentBoundEvent_2_DamageSig__DelegateSignature",
    function(this, amount_param)
        local time = os.clock()
        local dmg = amount_param:get()
        if (time - last_shield_damage_time < 0.01 and last_shield_damage == dmg) then return end
        last_shield_damage_time = time
        last_shield_damage = dmg
        -- lb:add_temp_static(dmg / 200, 8)
        printf("took %f shield damage", dmg)
    end)
    RegisterHook("/Game/Character/BP_PlayerCharacter.BP_PlayerCharacter_C:BndEvt__HealthComponent_K2Node_ComponentBoundEvent_1_HitSig__DelegateSignature",
    function(this, amount_param)
        local time = os.clock()
        local dmg = amount_param:get()
        if (time - last_health_damage_time < 0.01 and last_health_damage == dmg) then return end
        last_health_damage_time = time
        last_health_damage = dmg
        -- lb:add_temp_static(dmg / 100, 8)
        printf("took %f health damage", dmg)
    end)
end)

-- TODO: figure out if multiplayer is special
-- TODO: figure out if deepdives are special

--
-- Keybinds
--
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
    -- lb:set_vibration(next_val)
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
