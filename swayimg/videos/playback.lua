local M = {}

function M.new(S, U, V)
  local P = {}

  function P.sources_for(paths)
    local output = U.run_silent(S.bin .. " source " .. V.thumb_args(paths))
    if output == "" then
      return nil
    end
    return U.lines(output)
  end

  function P.manifest_for(paths)
    local output = U.run_silent(S.bin .. " manifest " .. V.thumb_args(paths))
    if output == "" then
      return nil
    end
    return output
  end

  function P.launch_mpv(manifest, sources, music)
    local script = io.open(S.mpv_script, "r")
    if not script then
      U.notify("Falta el script de mpv: " .. S.mpv_script)
      return false
    end
    script:close()
    local path = os.tmpname()
    local file = io.open(path, "w")
    if not file then
      U.notify("No se pudo crear el manifiesto de mpv.")
      return false
    end
    file:write(manifest)
    file:close()
    local command = "VIDEOCTL_MANIFEST=" .. U.q(path)
      .. " VIDEOCTL_BIN=" .. U.q(S.bin)
      .. " mpv --no-terminal --resume-playback=no --save-position-on-quit=no"
      .. " --script=" .. U.q(S.mpv_script) .. " --ytdl"
    if music then
      command = command .. " --no-video --audio-display=no --force-window=no"
    else
      command = command .. " --force-window=yes"
    end
    command = command .. " --"
    for _, source in ipairs(sources) do
      command = command .. " " .. U.q(source)
    end
    command = command .. " >/dev/null 2>&1 &"
    local ok = os.execute(command)
    if not ok then
      os.remove(path)
      U.notify("No se pudo iniciar mpv.")
      return false
    end
    return true
  end

  function P.play(separate)
    local paths = V.chosen_paths()
    if #paths == 0 or V.placeholder_selected(paths) then
      U.notify("No hay ningún vídeo seleccionado.")
      return
    end
    if separate and not S.music and #paths > 1 then
      local launched = 0
      for _, path in ipairs(paths) do
        local sources = P.sources_for({ path })
        local manifest = P.manifest_for({ path })
        if sources and manifest and P.launch_mpv(manifest, sources, false) then
          launched = launched + 1
        end
      end
      if launched > 0 then
        U.notify("Reproduciendo " .. tostring(launched) .. " vídeo(s).")
      else
        U.notify("No hay nada reproducible.")
      end
      return
    end
    local sources = P.sources_for(paths)
    local manifest = P.manifest_for(paths)
    if not sources or not manifest then
      U.notify("No hay nada reproducible.")
      return
    end
    if P.launch_mpv(manifest, sources, S.music) then
      local noun = S.music and "audio(s)." or "vídeo(s)."
      U.notify("Reproduciendo " .. tostring(#sources) .. " " .. noun)
    end
  end

  return P
end

return M
