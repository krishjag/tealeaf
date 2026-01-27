"""
Generate PAX Workflow Diagram
Creates a PNG showing JSON → PAX conversion and distribution workflow
"""

import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
from matplotlib.patches import FancyBboxPatch, FancyArrowPatch, Circle
import numpy as np

# Set up the figure with higher resolution
fig, ax = plt.subplots(1, 1, figsize=(20, 12.5))
ax.set_xlim(0, 16)
ax.set_ylim(0, 10)
ax.set_aspect('equal')
ax.axis('off')

# Colors
COLOR_JSON = '#F5A623'      # Orange for JSON
COLOR_PAX_TEXT = '#4A90D9'  # Blue for PAX text
COLOR_PAX_BIN = '#7B68EE'   # Purple for PAX binary
COLOR_LLM = '#50C878'       # Green for LLM
COLOR_STORAGE = '#FF6B6B'   # Red for Storage
COLOR_API = '#FFD700'       # Gold for APIs
COLOR_BG = '#1a1a2e'        # Dark background
COLOR_TEXT = '#FFFFFF'      # White text
COLOR_ARROW = '#888888'     # Gray arrows

# Background
fig.patch.set_facecolor(COLOR_BG)
ax.set_facecolor(COLOR_BG)

def draw_box(ax, x, y, width, height, color, label, sublabel=None, icon=None):
    """Draw a rounded box with label"""
    box = FancyBboxPatch((x - width/2, y - height/2), width, height,
                         boxstyle="round,pad=0.05,rounding_size=0.3",
                         facecolor=color, edgecolor='white', linewidth=2, alpha=0.9)
    ax.add_patch(box)

    if icon:
        ax.text(x, y + 0.15, icon, ha='center', va='center', fontsize=30, color=COLOR_TEXT)
        ax.text(x, y - 0.35, label, ha='center', va='center', fontsize=18, fontweight='bold', color=COLOR_TEXT)
    elif sublabel:
        # Two lines of text - position label higher and sublabel lower
        ax.text(x, y + 0.15, label, ha='center', va='center', fontsize=18, fontweight='bold', color=COLOR_TEXT)
        ax.text(x, y - 0.2, sublabel, ha='center', va='center', fontsize=15, color=COLOR_TEXT, alpha=0.8)
    else:
        # Single line - center it
        ax.text(x, y, label, ha='center', va='center', fontsize=20, fontweight='bold', color=COLOR_TEXT)

def draw_arrow(ax, start, end, color=COLOR_ARROW, style='->'):
    """Draw an arrow between two points"""
    ax.annotate('', xy=end, xytext=start,
                arrowprops=dict(arrowstyle=style, color=color, lw=2,
                               connectionstyle="arc3,rad=0"))

def draw_curved_arrow(ax, start, end, color=COLOR_ARROW, rad=0.2):
    """Draw a curved arrow (legacy, not used)"""
    ax.annotate('', xy=end, xytext=start,
                arrowprops=dict(arrowstyle='->', color=color, lw=2,
                               connectionstyle=f"arc3,rad={rad}"))

def draw_angled_arrow(ax, start, end, color=COLOR_ARROW, angle_first='horizontal'):
    """Draw an arrow with a right-angle bend"""
    x1, y1 = start
    x2, y2 = end
    if angle_first == 'horizontal':
        # Go horizontal first, then vertical
        mid_x, mid_y = x2, y1
    else:
        # Go vertical first, then horizontal
        mid_x, mid_y = x1, y2

    # Draw the two line segments
    ax.plot([x1, mid_x], [y1, mid_y], color=color, lw=2, solid_capstyle='round')
    ax.annotate('', xy=end, xytext=(mid_x, mid_y),
                arrowprops=dict(arrowstyle='->', color=color, lw=2))

# Title
ax.text(8, 9.5, 'PAX Format: JSON-Compatible Schema-Aware Serialization',
        ha='center', va='center', fontsize=24, fontweight='bold', color=COLOR_TEXT)
ax.text(8, 9.0, 'Lossless conversion with 65-80% size reduction',
        ha='center', va='center', fontsize=20, color=COLOR_TEXT, alpha=0.8)

# === LEFT SIDE: Source Data ===
draw_box(ax, 2, 6.5, 2.2, 1.2, COLOR_JSON, 'JSON Data', '36.8 KB', '{ }')

# === CENTER: PAX Conversion ===
draw_box(ax, 5.5, 6.5, 2.2, 1.2, COLOR_PAX_TEXT, 'PAX Text', '19.6 KB', '.pax')
draw_box(ax, 9, 6.5, 2.2, 1.2, COLOR_PAX_BIN, 'PAX Binary', '6.9 KB', '.paxb')

# Arrows for conversion flow
draw_arrow(ax, (3.2, 6.5), (4.3, 6.5), 'white')
draw_arrow(ax, (6.7, 6.5), (7.8, 6.5), 'white')

# Conversion labels (forward)
ax.text(3.75, 6.9, 'from-json', ha='center', va='center', fontsize=15, color=COLOR_TEXT, style='italic')
ax.text(7.25, 6.9, 'compile', ha='center', va='center', fontsize=15, color=COLOR_TEXT, style='italic')

# Reverse arrows (below the forward arrows)
draw_arrow(ax, (4.3, 6.1), (3.2, 6.1), 'white')  # PAX Text -> JSON
draw_arrow(ax, (7.8, 6.1), (6.7, 6.1), 'white')  # PAX Binary -> PAX Text

# Conversion labels (reverse)
ax.text(3.75, 5.7, 'to-json', ha='center', va='center', fontsize=15, color=COLOR_TEXT, style='italic')
ax.text(7.25, 5.7, 'decompile', ha='center', va='center', fontsize=15, color=COLOR_TEXT, style='italic')

# === RIGHT SIDE: Destinations ===
# LLM
draw_box(ax, 12.5, 7.8, 2.4, 1.0, COLOR_LLM, 'LLM APIs')

# Storage
draw_box(ax, 12.5, 6.5, 2.4, 1.0, COLOR_STORAGE, 'Storage')

# APIs
draw_box(ax, 12.5, 5.2, 2.4, 1.0, COLOR_API, 'REST APIs',)

# Arrows to destinations
# PAX Text (.pax) -> LLM APIs: from TOP of PAX, to LEFT of LLM
# PAX Text box: center (5.5, 6.5), height 1.2, so top edge = 6.5 + 0.6 = 7.1
draw_angled_arrow(ax, (5.5, 7.15), (11.3, 7.8), 'white', 'vertical')

# PAX Binary (.paxb) -> Storage: straight horizontal
draw_arrow(ax, (10.1, 6.5), (11.3, 6.5), 'white')

# PAX Binary (.paxb) -> REST APIs: from BOTTOM of PAXB, to LEFT of APIs
# PAX Binary box: center (9, 6.5), height 1.2, so bottom edge = 6.5 - 0.6 = 5.9
draw_angled_arrow(ax, (9, 5.85), (11.3, 5.2), 'white', 'vertical')

# === BOTTOM: Size Comparison Bar Chart ===
bar_y = 3.4
bar_height = 0.45
bar_spacing = 0.6  # Vertical spacing between bars
bar_left = 3.2  # Left edge of bars
label_x = 1.0   # X position for labels (left-aligned)
size_x = 3.0    # X position for size labels (right-aligned before bar)

# JSON bar (full width reference)
json_width = 4.2
ax.text(label_x, bar_y, 'JSON', ha='left', va='center', fontsize=16, fontweight='bold', color=COLOR_JSON)
ax.text(size_x, bar_y, '36.8 KB', ha='right', va='center', fontsize=16, fontweight='bold', color=COLOR_TEXT, alpha=0.9)
ax.add_patch(FancyBboxPatch((bar_left, bar_y - bar_height/2), json_width, bar_height,
                            boxstyle="round,pad=0.03,rounding_size=0.1",
                            facecolor=COLOR_JSON, edgecolor='white', linewidth=1, alpha=0.9))
ax.text(bar_left + json_width/2, bar_y, '100%', ha='center', va='center',
        fontsize=16, fontweight='bold', color=COLOR_TEXT)

# PAX Text bar
pax_text_width = json_width * 0.53  # 53% of JSON
ax.text(label_x, bar_y - bar_spacing, 'PAX Text', ha='left', va='center', fontsize=16, fontweight='bold', color=COLOR_PAX_TEXT)
ax.text(size_x, bar_y - bar_spacing, '19.6 KB', ha='right', va='center', fontsize=16, fontweight='bold', color=COLOR_TEXT, alpha=0.9)
ax.add_patch(FancyBboxPatch((bar_left, bar_y - bar_height/2 - bar_spacing), pax_text_width, bar_height,
                            boxstyle="round,pad=0.03,rounding_size=0.1",
                            facecolor=COLOR_PAX_TEXT, edgecolor='white', linewidth=1, alpha=0.9))
ax.text(bar_left + pax_text_width/2, bar_y - bar_spacing, '53%', ha='center', va='center',
        fontsize=16, fontweight='bold', color=COLOR_TEXT)

# PAX Binary bar
pax_bin_width = json_width * 0.19  # 19% of JSON
ax.text(label_x, bar_y - bar_spacing*2, 'PAX Binary', ha='left', va='center', fontsize=16, fontweight='bold', color=COLOR_PAX_BIN)
ax.text(size_x, bar_y - bar_spacing*2, '6.9 KB', ha='right', va='center', fontsize=16, fontweight='bold', color=COLOR_TEXT, alpha=0.9)
ax.add_patch(FancyBboxPatch((bar_left, bar_y - bar_height/2 - bar_spacing*2), pax_bin_width, bar_height,
                            boxstyle="round,pad=0.03,rounding_size=0.1",
                            facecolor=COLOR_PAX_BIN, edgecolor='white', linewidth=1, alpha=0.9))
ax.text(bar_left + pax_bin_width/2, bar_y - bar_spacing*2, '19%', ha='center', va='center',
        fontsize=16, fontweight='bold', color=COLOR_TEXT)

# Size comparison title
ax.text(4.5, 4.0, 'Size Comparison (Real Data: Retail Orders)', ha='center', va='center',
        fontsize=18, fontweight='bold', color=COLOR_TEXT)

# === RIGHT BOTTOM: Features ===
features_x = 10.5
features_y = 3.2

# Feature box
feature_box = FancyBboxPatch((8.5, 0.8), 7, 3.2,
                             boxstyle="round,pad=0.1,rounding_size=0.3",
                             facecolor='#2a2a4e', edgecolor='white', linewidth=2, alpha=0.8)
ax.add_patch(feature_box)

ax.text(12, 3.7, 'Key Features', ha='center', va='center',
        fontsize=16, fontweight='bold', color=COLOR_TEXT)

features = [
    ('*', 'Lossless Conversion', 'Full JSON round-trip fidelity'),
    ('+', 'Nested Structures', 'Unlimited object/array depth'),
    ('#', 'Schema Support', 'Type-safe with @struct/@table'),
    ('~', 'Auto Compression', 'ZLIB for large sections'),
    ('&', 'References', 'Deduplication via !refs'),
]

for i, (icon, title, desc) in enumerate(features):
    y_pos = 3.1 - i * 0.5
    ax.text(8.85, y_pos, icon, ha='center', va='center', fontsize=20, fontweight='bold', color=COLOR_LLM)
    ax.text(9.2, y_pos, title, ha='left', va='center', fontsize=18, fontweight='bold', color=COLOR_TEXT)
    ax.text(11.4, y_pos, f'— {desc}', ha='left', va='center', fontsize=18, color=COLOR_TEXT, alpha=0.8)

# === BOTTOM: Token savings callout ===
token_box = FancyBboxPatch((1, 0.35), 5.5, 1.0,
                           boxstyle="round,pad=0.1,rounding_size=0.2",
                           facecolor='#2a4a2a', edgecolor=COLOR_LLM, linewidth=2, alpha=0.9)
ax.add_patch(token_box)

ax.text(3.75, 0.85, 'LLM Token Savings', ha='center', va='center',
        fontsize=20, fontweight='bold', color=COLOR_LLM)
ax.text(3.75, 0.5, 'Schema-first design eliminates repeated field names', ha='center', va='center',
        fontsize=18, color=COLOR_TEXT, alpha=0.9)

# Footer
ax.text(8, 0.05, 'PAX v2.0-beta.1 — Peace between human and machine',
        ha='center', va='center', fontsize=18, color=COLOR_TEXT, alpha=0.5, style='italic')

# Save with high resolution
plt.tight_layout()
plt.savefig('examples/pax_workflow.png', dpi=300, facecolor=COLOR_BG,
            edgecolor='none', bbox_inches='tight', pad_inches=0.3)
plt.savefig('docs/pax_workflow.png', dpi=300, facecolor=COLOR_BG,
            edgecolor='none', bbox_inches='tight', pad_inches=0.3)
print("Saved: docs/pax_workflow.png")
