local M = {}

function M.new(S)
  local H = {}

  H.text = table.concat({
    "videoctl — teclas",
    "w  descargar  Shift+S  escanear  Shift+I  importar URL",
    "Enter  reproducir (marcados o actual)  Shift+Enter  ventanas separadas",
    "h/j/k/l  mover selección",
    "Space  marcar/desmarcar  Shift+Space  limpiar marcas",
    "Shift+M  modo música (solo audio, una instancia)",
    "/  filtrar por categoría  Ctrl+f  filtrar por directorio",
    "a  asignar categoría  Shift+C  gestionar categorías",
    "i  información  ?  esta ayuda",
    "y  copiar URL/carpeta  Shift+Y  copiar ambos",
    "d  quitar del visor  Shift+D  borrar metadata/fichero",
    "Shift+R  reparar miniaturas (ffmpeg/yt-dlp)",
    "r  recargar  t  texto  f  pantalla completa  +/-  tamaño  Esc  salir",
  }, "\n")

  function H.toggle()
    if S.help_visible then
      S.help_visible = false
      swayimg.text.visible = false
      return
    end
    S.help_visible = true
    swayimg.text.visible = true
    swayimg.text.status = H.text
  end

  return H
end

return M
