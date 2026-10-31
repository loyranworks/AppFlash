Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $root 'src-tauri\icons'
[IO.Directory]::CreateDirectory($directory) | Out-Null
$bitmap = [Drawing.Bitmap]::new(256, 256)
$graphics = [Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.Clear([Drawing.Color]::Transparent)
$graphics.ScaleTransform(2, 2)
$ink = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#587a6b'))
$outline = [Drawing.Drawing2D.GraphicsPath]::new()
$outline.AddArc(4, 4, 68, 68, 180, 90)
$outline.AddArc(56, 4, 68, 68, 270, 90)
$outline.AddArc(56, 56, 68, 68, 0, 90)
$outline.AddArc(4, 56, 68, 68, 90, 90)
$outline.CloseFigure()
$graphics.FillPath($ink, $outline)
$graphics.CompositingMode = [Drawing.Drawing2D.CompositingMode]::SourceCopy
$clear = [Drawing.SolidBrush]::new([Drawing.Color]::Transparent)
$flash = [Drawing.Drawing2D.GraphicsPath]::new()
$flash.AddPolygon([Drawing.PointF[]]@([Drawing.PointF]::new(70,26),[Drawing.PointF]::new(43,68),[Drawing.PointF]::new(62,68),[Drawing.PointF]::new(54,103),[Drawing.PointF]::new(89,56),[Drawing.PointF]::new(69,56),[Drawing.PointF]::new(78,26)))
$graphics.FillPath($clear, $flash)
$file = Join-Path $directory 'icon.png'
$bitmap.Save($file,[Drawing.Imaging.ImageFormat]::Png)
$bytes = [IO.File]::ReadAllBytes($file)
$stream = [IO.File]::Create((Join-Path $directory 'icon.ico'))
$writer = [IO.BinaryWriter]::new($stream)
$writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]1)
$writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0)
$writer.Write([uint16]1); $writer.Write([uint16]32); $writer.Write([uint32]$bytes.Length); $writer.Write([uint32]22)
$writer.Write($bytes); $writer.Dispose()
$graphics.Dispose(); $bitmap.Dispose(); $ink.Dispose(); $clear.Dispose(); $outline.Dispose(); $flash.Dispose()
