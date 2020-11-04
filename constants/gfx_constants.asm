TILE_WIDTH EQU 8 ; pixels
LEN_1BPP_TILE EQU 1 * TILE_WIDTH ; bytes
LEN_2BPP_TILE EQU 2 * TILE_WIDTH ; bytes

	const_def
	const SHADE_WHITE ; %00
	const SHADE_LIGHT ; %01
	const SHADE_DARK  ; %10
	const SHADE_BLACK ; %11
NUM_PAL_COLORS EQU const_value

PAL_COLOR_SIZE EQU 2
PALETTE_SIZE EQU NUM_PAL_COLORS * PAL_COLOR_SIZE

NUM_ACTIVE_PALS EQU 4

SCREEN_WIDTH  EQU 20
SCREEN_HEIGHT EQU 18
SCREEN_WIDTH_PX  EQU SCREEN_WIDTH  * TILE_WIDTH ; pixels
SCREEN_HEIGHT_PX EQU SCREEN_HEIGHT * TILE_WIDTH ; pixels

BG_MAP_WIDTH  EQU 32 ; tiles
BG_MAP_HEIGHT EQU 32 ; tiles

SPRITEBUFFERSIZE EQU 7 * 7 * LEN_1BPP_TILE

; DMGPalToGBCPal
CONVERT_BGP  EQU 0
CONVERT_OBP0 EQU 1
CONVERT_OBP1 EQU 2

; HP bar
HP_BAR_GREEN  EQU 0
HP_BAR_YELLOW EQU 1
HP_BAR_RED    EQU 2

; hAutoBGTransferEnabled
TRANSFERTOP    EQU 0
TRANSFERMIDDLE EQU 1
TRANSFERBOTTOM EQU 2

; hRedrawRowOrColumnMode
REDRAW_COL EQU 1
REDRAW_ROW EQU 2

; tile list ids
; TileIDListPointerTable indexes (see data/tilemaps.asm)
	const_def
	const TILEMAP_MON_PIC
	const TILEMAP_SLIDE_DOWN_MON_PIC_7X5
	const TILEMAP_SLIDE_DOWN_MON_PIC_7X3
	const TILEMAP_GENGAR_INTRO_1
	const TILEMAP_GENGAR_INTRO_2
	const TILEMAP_GENGAR_INTRO_3
	const TILEMAP_GAME_BOY
	const TILEMAP_LINK_CABLE
