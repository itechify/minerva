---
status: accepted
---

# The Deal Finder finds the Player Shop in the client area

A player browsing Player Shops wants to know at a glance which listings are deals: priced below what they know the item sells for. They already keep that on their Market Lines (ADR 0011). The Deal Finder compares each listing of the open shop with one of them, the Deal Line, and marks the deals beside the shop.

The shop's window opens at the same spot every time for a given size of the game window, centred, and the player can drag it. Its listings would be read from a region marked at setup, as the minimap title is (the Title Region), or found in a capture of the whole client area, as the Item Tooltip is (ADR 0007).

So the Deal Finder finds the shop itself: while it is on, the shell captures the game's whole client area a few times a second (every 400 ms), and the core finds the shop's listings in it and reads them.

## Finding the listings

- The game draws its UI at a size it picks from a setting and the window's size, the UI Size: the reader finds the list at any, and reads it at the samples' (ADR 0020). The sizes below are the samples'. Each listing is a light blue-grey band (202,213,223) exactly 253 px wide, with the item's name on one line and "1 for 999 mesos" 27 px below it. The list is where many scanline runs of that colour, 253 px long, start at one column. Nothing else in the game's UI draws such a stack.
- Live captures show the game's colours a little darker than screenshots do (`prep.rs`), so the band is matched at any brightness, as the minimap's panel is.
- The text is black blended into the band, so every pixel of a clean line lies between the band's colour and black. A line with a pixel off that line is covered: by the game's pointer, by a window of ours, by the selection. Its listing reads as "?", never misread.
- A sold-out listing is greyed: its band isn't the listing colour, so it isn't read. An empty slot is a band with no text. The listing the player selected is orange (251,167,46) from 7 px into its band, its text black as on the others: it is read the same way, against its own colour.
- The list's slots lie on a grid, 63 and 62 px apart in turn, through every listing read clean. A listing that isn't clean is covered, and counts only in its slot on the grid: under the pointer, pieces of two listings' lines can pair up between slots. The reader remembers the slots it read a listing clean in while the list stays where it is. A slot with no listing now is reported covered only when something dark is on it, the pointer's outline or the dark slate fill of the game's tooltip by it: a sold-out listing holds nothing dark, and live captures show it near white (its band (242,245,248), its rules white), far lighter than screenshots do, so no check against its colours or against white holds. Another shop's window can open a few px higher or lower where the last one was: slots off the grid of the listings read clean now are the last list's and are forgotten. Both came from a Bug Report's crops, which are samples now. The core keeps what was read there before, and its badge says so, while the listings read since say the list is as it was: none reads otherwise where it was, and none read before is gone uncovered. Otherwise (scrolled, a listing bought or sold out, the shop moved) a covered listing is "?". Players hover the listing they look at, so a verdict that vanished under the pointer would vanish when it matters.
- ocrs reads every line of the list at once (ADR 0002), and the reader remembers what each line's ink read as, so a shop left open costs a scan of the capture (about 7 ms) and no recognition. A price counts only in the game's exact shape: "<quantity> for <mesos> mesos", the mesos grouped in threes with commas.

## Judging a listing

- The price compared is the price of one: the listing's mesos over its quantity. A throwing star or a bullet (an item OSMS gives a recharge price, `unitPrice`) is one set however many it holds: its listing's number is the set's count, so its whole price is compared, as players price a set. It is a deal when it is below the Deal Line's price for the item by at least the Deal Margin, a percentage in the settings (0 by default: any price below).
- The name is matched against OSMS's item files as the Item Card matches a tooltip's (ADR 0010). A name the game cut short ("Pet Equip Speed Scroll: L...") is the one item whose name starts so; several, and the listing is "?". Several items sharing the name that don't look alike (their kinds or shop prices differ) are "?" too.
- Equipment is judged by its name alone: Market Lines keep one price per item, whatever its stats, and the listing doesn't show them.
- Only the listings on screen are read. Minerva sends no input to the game (ADR 0001), so the player scrolls, and the badges follow within half a second.

## Considered options

- **A Shop Region marked at setup.** Setup would grow a step that only shop browsers use, and would be asked for again whenever the game window changes size, as the Title Region is. A shop dragged elsewhere would be read wrong, or not at all, without a word. Finding the shop costs little more than reading a region: the scan of the capture is a few milliseconds, and recognition, the costly part, is the same either way.
- **Recognising text across the whole client area.** ocrs's detection model isn't shipped (ADR 0002), and reading every line on screen would cost far more than finding the bands, with chat and names to tell apart from listings.
- **Reading each item's Item Tooltip.** It would tell equipment's stats, but takes the player's hover over each listing, one at a time: the Item Card does that already.
- **Scrolling the list for the player.** Sending input to the game is ruled out (ADR 0001).

## Consequences

- While the Deal Finder is on, the game's whole client area is captured every 400 ms, on the reader thread, as the Item Tooltip is at a press. It is off at every launch, so a forgotten Deal Finder never captures across launches.
- The Deal Badges' window sits left of the shop's window (right of it with no room on the left), never over the listings it reads, and it moves out of an Item Card or Item Lookup press's capture, as the Item Card does.
- A reading is an input (ADR 0009): `Event::PlayerShop` writes it down, the same as the last one written as such, and a replay takes it in again. Its crop is the list alone, with none of the shop's title, its seller's name or its chat, as `samples/player-shop` keeps its frames.
- Hired Merchants aren't read. If their window is laid out as the Player Shop's is, the reader reads them as they are; if not, they need anchors of their own.
