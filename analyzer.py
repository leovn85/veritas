import os
import glob
import json
import pandas as pd
import tkinter as tk
from tkinter import ttk, font
import matplotlib.pyplot as plt
from matplotlib.backends.backend_tkagg import FigureCanvasTkAgg
from matplotlib.figure import Figure

class InteractiveAnalyzer:
    def __init__(self, master):
        self.master = master
        master.title("Honkai: Star Rail - Damage Analyzer")
        master.geometry("1200x900")

        self.summaries_data = self.load_all_summaries()
        
        self.create_widgets()
        if self.summaries_data:
            self.create_comparison_plots()

    def load_all_summaries(self, directory="battle_summaries"):
        summary_files = glob.glob(os.path.join(directory, "SUMMARY_*.json"))
        if not summary_files:
            print(f"No summary files found in '{directory}'.")
            return None

        print(f"Found {len(summary_files)} summary files. Processing...")
        
        all_data = []
        for filepath in summary_files:
            try:
                filename = os.path.basename(filepath)
                parts = filename[:-5].split('_')
                
                session_id = filename
                timestamp_str = "0"
                
                if len(parts) >= 6:
                    team_name = parts[1]
                    battle_mode = parts[2]
                    time_part = parts[5]
                    date_part = parts[4]
                    date_str = f"{date_part[0:4]}-{date_part[4:6]}-{date_part[6:8]}"
                    time_str = f"{time_part[0:2]}:{time_part[2:4]}:{time_part[4:6]}"
                    session_id = f"{team_name} [{battle_mode}] - {date_str} {time_str}"
                    timestamp_str = f"{date_part}{time_part}" 
                    
                with open(filepath, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                    
                    all_data.append({
                        "filepath": filepath,
                        "session_id": session_id,
                        "timestamp": timestamp_str,
                        "total_dpav": data.get("total_dpav", 0),
                        "total_av": data.get("total_av", 0)
                    })
            except Exception as e:
                print(f"Error processing file {filepath}: {e}")
        if all_data:
            all_data.sort(key=lambda x: x['timestamp'], reverse=True)
        return all_data

    def create_widgets(self):
        main_frame = ttk.Frame(self.master, padding=10)
        main_frame.pack(fill=tk.BOTH, expand=True)

        list_frame = ttk.LabelFrame(main_frame, text="Battle Sessions", padding=10)
        list_frame.pack(fill=tk.X, pady=(0, 10))

        if not self.summaries_data:
            ttk.Label(list_frame, text="No summary files found.").pack()
            return

        canvas = tk.Canvas(list_frame, height=100, bg='white', highlightthickness=0)
        scrollbar = ttk.Scrollbar(list_frame, orient="vertical", command=canvas.yview)
        scrollable_frame = ttk.Frame(canvas)
        
        style = ttk.Style()
        style.configure("White.TFrame", background="white")
        scrollable_frame.configure(style="White.TFrame")


        scrollable_frame.bind(
            "<Configure>",
            lambda e: canvas.configure(scrollregion=canvas.bbox("all"), width=e.width)
        )

        canvas.create_window((0, 0), window=scrollable_frame, anchor="nw")
        canvas.configure(yscrollcommand=scrollbar.set)

        def _on_mousewheel(event):
            # For Windows/Linux
            canvas.yview_scroll(int(-1*(event.delta/120)), "units")
        canvas.bind_all("<MouseWheel>", _on_mousewheel)


        style.configure("Hyperlink.TLabel", foreground="blue", background="white")
        hyperlink_font = font.Font(family="Segoe UI", size=9, underline=True)

        num_columns = 2
        for i, summary in enumerate(self.summaries_data):
            row = i // num_columns
            col = i % num_columns
            link_text = f"View Detail - {os.path.basename(summary['filepath'])}"
            link = ttk.Label(scrollable_frame, text=link_text, style="Hyperlink.TLabel", cursor="hand2", font=hyperlink_font)
            link.grid(row=row, column=col, padx=10, pady=3, sticky="w")
            link.bind("<Button-1>", lambda e, s=summary: self.show_detail_window(s))
            scrollable_frame.grid_columnconfigure(col, weight=1)


        canvas.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

        plot_frame = ttk.LabelFrame(main_frame, text="Overall Comparison", padding=10)
        plot_frame.pack(fill=tk.BOTH, expand=True)

        self.fig_comparison = Figure(figsize=(10, 8), dpi=100)
        self.canvas_comparison = FigureCanvasTkAgg(self.fig_comparison, master=plot_frame)
        self.canvas_comparison.get_tk_widget().pack(fill=tk.BOTH, expand=True)

    def create_comparison_plots(self):
        df = pd.DataFrame(self.summaries_data)
        self.fig_comparison.clear()
        
        ax1, ax2 = self.fig_comparison.subplots(2, 1)
        
        window_title = "Honkai: Star Rail - Damage Analyzer"

        self.fig_comparison.suptitle(window_title, fontsize=16, weight='bold')

        df_dpav_sorted = df.sort_values(by="total_dpav", ascending=False)
        best_dpav_session = df_dpav_sorted.iloc[0]['session_id'] if not df_dpav_sorted.empty else None
        dpav_colors = ['gold' if session == best_dpav_session else 'skyblue' for session in df_dpav_sorted["session_id"]]
        
        labels_dpav = [s.replace(' - ', '\n', 1) for s in df_dpav_sorted["session_id"]]
        
        ax1.bar(labels_dpav, df_dpav_sorted["total_dpav"], color=dpav_colors)
        ax1.set_title("Performance Comparison (DpAV - Higher is Better)", fontsize=12)
        ax1.set_ylabel("Damage Per Action Value (DpAV)")
        ax1.tick_params(axis='x', rotation=40, labelsize=9)
        if not df_dpav_sorted.empty:
            max_dpav = df_dpav_sorted["total_dpav"].max()
            ax1.set_ylim(top=max_dpav * 1.3)
        for i, v in enumerate(df_dpav_sorted["total_dpav"]):
            ax1.text(i, v, f"{v:,.0f}", ha='center', va='bottom', fontsize=9)

        # --- AV plot with highlight ---
        df_av_sorted = df.sort_values(by="total_av", ascending=True)
        best_av_session = df_av_sorted.iloc[0]['session_id'] if not df_av_sorted.empty else None
        av_colors = ['limegreen' if session == best_av_session else 'lightcoral' for session in df_av_sorted["session_id"]]

        labels_av = [s.replace(' - ', '\n', 1) for s in df_av_sorted["session_id"]]

        ax2.bar(labels_av, df_av_sorted["total_av"], color=av_colors)
        ax2.set_title("Total Action Value Comparison (AV - Lower is Better)", fontsize=12)
        ax2.set_ylabel("Total Action Value (AV)")
        ax2.tick_params(axis='x', rotation=40, labelsize=9)
        if not df_av_sorted.empty:
            max_av = df_av_sorted["total_av"].max()
            ax2.set_ylim(top=max_av * 1.3)
        for i, v in enumerate(df_av_sorted["total_av"]):
            ax2.text(i, v, f"{v:.2f}", ha='center', va='bottom', fontsize=9)

        self.fig_comparison.tight_layout(pad=3.0, rect=[0, 0, 1, 0.96])
        self.canvas_comparison.draw()

    def show_detail_window(self, summary_info):
        try:
            with open(summary_info['filepath'], 'r', encoding='utf-8') as f:
                data = json.load(f)
        except Exception as e:
            print(f"Could not open detail file: {e}")
            return

        detail_window = tk.Toplevel(self.master)
        detail_window.title(f"Team Detail: {summary_info['session_id']}")
        detail_window.geometry("800x600")

        fig_detail = Figure(figsize=(8, 6), dpi=100)
        canvas_detail = FigureCanvasTkAgg(fig_detail, master=detail_window)
        canvas_detail.get_tk_widget().pack(fill=tk.BOTH, expand=True)

        ax1, ax2 = fig_detail.subplots(1, 2)
        fig_detail.suptitle(f"Battle Analysis for {data.get('team_name', 'Unknown')} Team", fontsize=14)

        characters = data.get("characters", {})
        char_names = list(characters.keys())
        char_damages = [d['total_damage'] for d in characters.values()]

        ax1.pie(char_damages, labels=char_names, autopct='%1.1f%%', startangle=90)
        ax1.set_title("Damage Distribution")
        ax1.axis('equal')

        ax2.bar(char_names, char_damages, color='teal')
        ax2.set_title("Total Damage by Character")
        ax2.set_ylabel("Total Damage")
        ax2.tick_params(axis='x', rotation=45, labelsize=9)
        if char_damages:
            max_dmg = max(char_damages)
            ax2.set_ylim(top=max_dmg * 1.3)
        for i, v in enumerate(char_damages):
            ax2.text(i, v, f"{v:,.0f}", ha='center', va='bottom', fontsize=9)

        fig_detail.tight_layout(pad=3.0, rect=[0, 0, 1, 0.95])
        canvas_detail.draw()

if __name__ == "__main__":
    root = tk.Tk()
    try:
        style = ttk.Style(root)
        style.theme_use('clam')
    except tk.TclError:
        pass
    app = InteractiveAnalyzer(root)
    root.mainloop()