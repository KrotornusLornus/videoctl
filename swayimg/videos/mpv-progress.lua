local utils = require("mp.utils")
local msg = require("mp.msg")

local manifest_path = os.getenv("VIDEOCTL_MANIFEST") or mp.get_opt("videoctl-manifest")
local bin = os.getenv("VIDEOCTL_BIN") or mp.get_opt("videoctl-bin") or "videoctl"
local items = {}
local current = nil
local last_position = 0
local last_duration = nil

local function load_manifest()
  if not manifest_path then
    return
  end
  local file, read_error = io.open(manifest_path, "r")
  if not file then
    msg.warn("no se pudo leer el manifiesto: " .. tostring(read_error))
    return
  end
  local data = file:read("*a")
  file:close()
  if not data then
    msg.warn("no se pudo leer el manifiesto")
    return
  end
  local parsed, parse_error = utils.parse_json(data)
  if not parsed then
    msg.warn("manifiesto inválido: " .. tostring(parse_error))
    return
  end
  if type(parsed.items) == "table" then
    items = parsed.items
  end
end

local function item_for_current_file()
  local position = mp.get_property_number("playlist-pos", 0)
  if position == nil or position < 0 then
    position = 0
  end
  local item = items[position + 1]
  if not item and #items == 1 then
    item = items[1]
  end
  return item
end

local function report(item, force_watched)
  if type(item) ~= "table" or item.id == nil then
    return
  end
  local position = last_position or 0
  local duration = last_duration
  if duration == nil or duration <= 0 then
    duration = item.duration
  end
  local args = {
    bin,
    "progress",
    "--id",
    tostring(item.id),
    "--pos",
    string.format("%.3f", position),
  }
  if duration ~= nil and duration > 0 then
    table.insert(args, "--duration")
    table.insert(args, string.format("%.3f", duration))
  end
  if force_watched then
    table.insert(args, "--watched")
  end
  local result = utils.subprocess({ args = args, playback_only = false })
  if result and result.status ~= 0 then
    msg.warn("videoctl progress falló: " .. tostring(result.stderr or result.status))
  end
end

local function on_file_loaded()
  current = item_for_current_file()
  last_position = 0
  last_duration = mp.get_property_number("duration")
  if type(current) ~= "table" then
    return
  end
  report(current, false)
  local resume = tonumber(current.resume) or 0
  if resume > 5 then
    local loaded = current
    mp.add_timeout(0.4, function()
      if current ~= loaded then
        return
      end
      local duration = mp.get_property_number("duration") or tonumber(current.duration) or 0
      if duration <= 0 or resume < duration - 5 then
        mp.commandv("seek", string.format("%.3f", resume), "absolute")
      end
    end)
  end
end

local function on_end_file(event)
  if type(current) == "table" then
    report(current, event.reason == "eof")
  end
end

local function on_shutdown()
  if type(current) == "table" then
    report(current, false)
  end
  if manifest_path then
    os.remove(manifest_path)
  end
end

load_manifest()
mp.observe_property("time-pos", "number", function(_, value)
  if value ~= nil then
    last_position = value
  end
end)
mp.observe_property("duration", "number", function(_, value)
  if value ~= nil and value > 0 then
    last_duration = value
  end
end)
mp.register_event("file-loaded", on_file_loaded)
mp.register_event("end-file", on_end_file)
mp.register_event("shutdown", on_shutdown)
mp.add_periodic_timer(10, function()
  if type(current) == "table" then
    report(current, false)
  end
end)
