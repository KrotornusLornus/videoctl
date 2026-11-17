local image_config = (os.getenv("XDG_CONFIG_HOME") or (os.getenv("HOME") .. "/.config")) .. "/swayimg/init.lua"
pcall(dofile, image_config)

local home = os.getenv("HOME") or ""
local config_home = os.getenv("XDG_CONFIG_HOME") or (home .. "/.config")
local config_source = ""
if type(debug) == "table" and type(debug.getinfo) == "function" then
  local info = debug.getinfo(1, "S")
  if info and info.source then
    config_source = info.source
  end
end
if config_source:sub(1, 1) == "@" then
  config_source = config_source:sub(2)
end
local config_dir = config_source:match("^(.*)/[^/]+$") or (config_home .. "/swayimg/videos")

local S = {
  bin = os.getenv("VIDEOCTL_BIN") or "videoctl",
  home = home,
  config_dir = config_dir,
  mpv_script = os.getenv("VIDEOCTL_MPV_SCRIPT") or (config_dir .. "/mpv-progress.lua"),
  default_root = home .. "/Videos",
  filter_from_env = false,
  music = os.getenv("VIDEOCTL_MUSIC") == "1",
  selected = {},
  selected_order = {},
  help_visible = false,
}

local env_root = os.getenv("VIDEOCTL_ROOT")
S.root = (env_root and env_root ~= "") and env_root or S.default_root
S.filter = { kind = "dir", value = S.root }

local env_kind = os.getenv("VIDEOCTL_FILTER_KIND")
local env_value = os.getenv("VIDEOCTL_FILTER_VALUE")
if env_kind == "all" then
  S.filter = { kind = "all", value = "" }
  S.filter_from_env = true
elseif env_kind == "category" and env_value and env_value ~= "" then
  S.filter = { kind = "category", value = env_value }
  S.filter_from_env = true
elseif env_kind == "dir" and env_value and env_value ~= "" then
  S.filter = { kind = "dir", value = env_value }
  S.filter_from_env = true
end

local function load(name)
  return dofile(config_dir .. "/" .. name .. ".lua")
end

local U = load("util").new(S)
local F = load("fuzzel").new(U)
local V = load("view").new(S, U)
local A = load("selection").new(S, U, V)
local P = load("playback").new(S, U, V)
local C = load("catalog").new(S, U, F, V)
local D = load("download").new(S, U, F, V)
local G = load("manage").new(S, U, F, V)
local T = load("thumbnail").new(S, U, F, V)
local H = load("help").new(S)
local K = load("keys").new(S, U, V, A, P, C, D, G, H, T)

swayimg.mode = "gallery"
swayimg.imagelist.order = "none"
swayimg.imagelist.reverse = false
swayimg.imagelist.recursive = false
swayimg.imagelist.adjacent = false
swayimg.imagelist.fsmon = false
swayimg.text.visible = true
swayimg.text.timeout = 0
swayimg.text.status_timeout = 0
swayimg.gallery.text = { topleft = {}, topright = {}, bottomleft = {}, bottomright = {} }
swayimg.gallery.thumb_size = 220
swayimg.gallery.aspect = "fill"
swayimg.gallery.padding_size = 6
swayimg.gallery.border_size = 5
swayimg.gallery.preload = true
swayimg.title = "videoctl — cargando…"

swayimg.on_initialized(function()
  swayimg.viewer.set_window_background(0x00000000)
  swayimg.viewer.set_image_background(0x00000000)
  swayimg.gallery.window_color = 0x00000000
  local ok = pcall(function()
    swayimg.defer(0.05, V.bootstrap)
  end)
  if not ok then
    V.bootstrap()
  end
end)

K.bind()
