import os
import glob
import json
import pandas as pd
import tkinter as tk
from tkinter import ttk, messagebox
import matplotlib.pyplot as plt
from matplotlib.backends.backend_tkagg import FigureCanvasTkAgg
from matplotlib.figure import Figure

# Sử dụng theme tối giản, hiện đại
plt.style.use('seaborn-v0_8-darkgrid' if 'seaborn-v0_8-darkgrid' in plt.style.available else 'default')


class ModernDamageAnalyzer:
    def __init__(self, master):
        self.master = master
        master.title("Honkai: Star Rail - Advanced Battle & Wave Analyzer")
        master.geometry("1440x940")
        master.minsize(1100, 740)

        self.summaries_data = self.load_all_summaries()
        self.current_filtered_data = list(self.summaries_data)
        self.sort_column = "timestamp"
        self.sort_descending = True

        self.create_layout()
        if self.summaries_data:
            self.refresh_table(self.current_filtered_data)
            self.create_comparison_plots(self.current_filtered_data)

    def load_all_summaries(self, directory="battle_summaries"):
        summary_files = glob.glob(os.path.join(directory, "SUMMARY_*.json"))
        if not summary_files:
            return []

        all_data = []
        for filepath in summary_files:
            try:
                filename = os.path.basename(filepath)
                with open(filepath, 'r', encoding='utf-8') as f:
                    data = json.load(f)

                # 1. ĐỌC TRỰC TIẾP TỪ JSON (Ghép tên toàn bộ thành viên trong lineup)
                lineup_list = data.get("lineup", [])
                if lineup_list and isinstance(lineup_list, list):
                    team_display = " / ".join(str(c) for c in lineup_list)
                else:
                    team_display = data.get("team_name", "Unknown")
                mode_str = data.get("battle_mode")
                stage_str = str(data.get("stage_id", "Unknown"))
                timestamp_raw = data.get("timestamp", "")
                total_dpav = float(data.get("total_dpav", 0.0) or 0.0)
                total_av = float(data.get("total_av", 0.0) or 0.0)
                total_dmg = float(data.get("total_damage", 0.0) or 0.0)

                # 2. FALLBACK TÁCH TÊN FILE (Nếu là file JSON cũ chưa có metadata)
                if not mode_str or stage_str == "Unknown" or not timestamp_raw:
                    stem = filename[:-5] if filename.endswith(".json") else filename
                    parts = stem.split('_')
                    if len(parts) >= 6:
                        time_part = parts[-1]
                        date_part = parts[-2]
                        stage_str = parts[-3].replace("Stage", "")
                        middle = parts[1:-3]
                        if not team_display or team_display == "Unknown":
                            team_display = middle[0]
                        if not mode_str:
                            mode_str = "_".join(middle[1:])
                        timestamp_raw = f"{date_part}_{time_part}"

                mode_str = mode_str if mode_str else "Other"

                # Format ngày giờ hiển thị
                date_str, time_str = "2026-01-01", "00:00:00"
                sort_ts = "0"
                if "_" in timestamp_raw:
                    d_p, t_p = timestamp_raw.split('_', 1)
                    if len(d_p) == 8 and len(t_p) >= 6:
                        date_str = f"{d_p[0:4]}-{d_p[4:6]}-{d_p[6:8]}"
                        time_str = f"{t_p[0:2]}:{t_p[2:4]}:{t_p[4:6]}"
                        sort_ts = f"{d_p}{t_p[:6]}"
                else:
                    sort_ts = str(timestamp_raw)

                session_display = f"{team_display} [{mode_str} - {stage_str}] ({date_str} {time_str})"

                all_data.append({
                    "filepath": filepath,
                    "filename": filename,
                    "session_id": session_display,
                    "team_name": team_display,
                    "battle_mode": mode_str,
                    "stage_id": stage_str,
                    "datetime": f"{date_str} {time_str}",
                    "timestamp": sort_ts,
                    "total_dpav": total_dpav,
                    "total_av": total_av,
                    "total_damage": total_dmg,
                    "raw_data": data
                })
            except Exception as e:
                print(f"[!] Error loading {filepath}: {e}")

        all_data.sort(key=lambda x: x['timestamp'], reverse=True)
        return all_data

    def reload_summaries(self):
        """Nạp lại dữ liệu từ thư mục battle_summaries mà không cần khởi động lại app."""
        self.summaries_data = self.load_all_summaries()
        modes = ["All"] + sorted(list({x["battle_mode"] for x in self.summaries_data}))
        teams = ["All"] + sorted(list({x["team_name"] for x in self.summaries_data}))
        stages = ["All"] + sorted(list({x["stage_id"] for x in self.summaries_data}))

        self.mode_cb["values"] = modes
        self.team_cb["values"] = teams
        self.stage_cb["values"] = stages

        if self.mode_var.get() not in modes:
            self.mode_var.set("All")
        if self.team_var.get() not in teams:
            self.team_var.set("All")
        if self.stage_var.get() not in stages:
            self.stage_var.set("All")

        self.apply_filters()

    def create_layout(self):
        main_pane = ttk.PanedWindow(self.master, orient=tk.VERTICAL)
        main_pane.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)

        # --- Top Section: Filters & Table ---
        top_frame = ttk.LabelFrame(main_pane, text="Battle Sessions History (Ctrl/Shift+Click to select multiple runs)", padding=8)
        main_pane.add(top_frame, weight=1)

        filter_bar = ttk.Frame(top_frame)
        filter_bar.pack(fill=tk.X, pady=(0, 6))

        ttk.Label(filter_bar, text="Mode:").pack(side=tk.LEFT, padx=(0, 4))
        modes = ["All"] + sorted(list({x["battle_mode"] for x in self.summaries_data}))
        self.mode_var = tk.StringVar(value="All")
        self.mode_cb = ttk.Combobox(filter_bar, textvariable=self.mode_var, values=modes, state="readonly", width=15)
        self.mode_cb.pack(side=tk.LEFT, padx=(0, 12))
        self.mode_cb.bind("<<ComboboxSelected>>", self.apply_filters)

        ttk.Label(filter_bar, text="Stage:").pack(side=tk.LEFT, padx=(0, 4))
        stages = ["All"] + sorted(list({x["stage_id"] for x in self.summaries_data}))
        self.stage_var = tk.StringVar(value="All")
        self.stage_cb = ttk.Combobox(filter_bar, textvariable=self.stage_var, values=stages, state="readonly", width=12)
        self.stage_cb.pack(side=tk.LEFT, padx=(0, 12))
        self.stage_cb.bind("<<ComboboxSelected>>", self.apply_filters)

        ttk.Label(filter_bar, text="Team:").pack(side=tk.LEFT, padx=(0, 4))
        teams = ["All"] + sorted(list({x["team_name"] for x in self.summaries_data}))
        self.team_var = tk.StringVar(value="All")
        self.team_cb = ttk.Combobox(filter_bar, textvariable=self.team_var, values=teams, state="readonly", width=16)
        self.team_cb.pack(side=tk.LEFT, padx=(0, 12))
        self.team_cb.bind("<<ComboboxSelected>>", self.apply_filters)

        btn_reload = ttk.Button(filter_bar, text="🔄 Reload Data", command=self.reload_summaries)
        btn_reload.pack(side=tk.LEFT, padx=4)

        btn_view_detail = ttk.Button(filter_bar, text="📊 View Run Details & Wave Breakdown", command=self.open_selected_detail)
        btn_view_detail.pack(side=tk.RIGHT, padx=4)

        btn_compare_multi = ttk.Button(filter_bar, text="⚖️ Compare Selected Runs", command=self.open_multi_run_comparison)
        btn_compare_multi.pack(side=tk.RIGHT, padx=4)

        # Table Treeview (selectmode="extended" cho phép chọn nhiều dòng để so sánh)
        columns = ("datetime", "team", "mode", "stage", "dpav", "av", "damage")
        self.tree = ttk.Treeview(top_frame, columns=columns, show="headings", height=6, selectmode="extended")

        headers = {
            "datetime": "Time", "team": "Team Lineup", "mode": "Mode",
            "stage": "Stage", "dpav": "DpAV", "av": "Total AV", "damage": "Total Damage"
        }
        for col_id, text in headers.items():
            self.tree.heading(col_id, text=text, command=lambda c=col_id: self.sort_by_column(c))

        self.tree.column("datetime", width=135, anchor="center")
        self.tree.column("team", width=280, anchor="w")
        self.tree.column("mode", width=130, anchor="center")
        self.tree.column("stage", width=90, anchor="center")
        self.tree.column("dpav", width=100, anchor="e")
        self.tree.column("av", width=85, anchor="e")
        self.tree.column("damage", width=130, anchor="e")

        tree_scroll = ttk.Scrollbar(top_frame, orient="vertical", command=self.tree.yview)
        self.tree.configure(yscrollcommand=tree_scroll.set)
        self.tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        tree_scroll.pack(side=tk.RIGHT, fill=tk.Y)
        self.tree.bind("<Double-1>", lambda e: self.open_selected_detail())

        # --- Bottom Section: Overview Plots ---
        bottom_frame = ttk.LabelFrame(main_pane, text="Performance Comparison Across Runs (Top 15 Displayed)", padding=8)
        main_pane.add(bottom_frame, weight=3)

        self.fig_comparison = Figure(figsize=(12, 6), dpi=100)
        self.canvas_comparison = FigureCanvasTkAgg(self.fig_comparison, master=bottom_frame)
        self.canvas_comparison.get_tk_widget().pack(fill=tk.BOTH, expand=True)

    def refresh_table(self, data):
        self.tree.delete(*self.tree.get_children())
        for idx, item in enumerate(data):
            self.tree.insert("", "end", iid=str(idx), values=(
                item["datetime"],
                item["team_name"],
                item["battle_mode"],
                item["stage_id"],
                f"{item['total_dpav']:,.1f}",
                f"{item['total_av']:.2f}",
                f"{item['total_damage']:,.0f}"
            ))

    def apply_filters(self, *args):
        m = self.mode_var.get()
        s = self.stage_var.get()
        t = self.team_var.get()

        filtered = self.summaries_data
        if m != "All":
            filtered = [x for x in filtered if x["battle_mode"] == m]
        if s != "All":
            filtered = [x for x in filtered if x["stage_id"] == s]
        if t != "All":
            filtered = [x for x in filtered if x["team_name"] == t]

        self.current_filtered_data = list(filtered)
        self.sort_data_in_place(self.current_filtered_data)
        self.refresh_table(self.current_filtered_data)
        self.create_comparison_plots(self.current_filtered_data)

    def sort_data_in_place(self, data_list):
        col = self.sort_column

        def sort_key(item):
            if col in ["dpav", "total_dpav"]:
                return item["total_dpav"]
            if col in ["av", "total_av"]:
                return item["total_av"]
            if col in ["damage", "total_damage"]:
                return item["total_damage"]
            if col in ["datetime", "timestamp"]:
                return item["timestamp"]
            if col == "team":
                return item["team_name"]
            if col == "mode":
                return item["battle_mode"]
            if col == "stage":
                return item["stage_id"]
            return item.get(col, "")

        data_list.sort(key=sort_key, reverse=self.sort_descending)

    def sort_by_column(self, col):
        if self.sort_column == col:
            self.sort_descending = not self.sort_descending
        else:
            self.sort_column = col
            self.sort_descending = True

        self.sort_data_in_place(self.current_filtered_data)
        self.refresh_table(self.current_filtered_data)
        self.create_comparison_plots(self.current_filtered_data)

    def create_comparison_plots(self, data):
        self.fig_comparison.clear()
        if not data:
            self.canvas_comparison.draw()
            return

        plot_data = data[:15]
        df = pd.DataFrame(plot_data)

        ax1, ax2 = self.fig_comparison.subplots(1, 2)

        # 1. DpAV Plot
        df_dpav = df.sort_values(by="total_dpav", ascending=True)
        y_labels = [f"{r['team_name']} [{r['battle_mode']}]\n{r['datetime'][5:]}" for _, r in df_dpav.iterrows()]
        best_dpav = df_dpav['total_dpav'].max()
        colors_dpav = ['#f39c12' if v == best_dpav else '#3498db' for v in df_dpav["total_dpav"]]

        bars1 = ax1.barh(y_labels, df_dpav["total_dpav"], color=colors_dpav, height=0.6)
        ax1.set_title("Performance Comparison (DpAV - Higher is better)", fontsize=11, fontweight="bold")
        ax1.set_xlabel("Damage per Action Value (DpAV)")
        ax1.tick_params(axis='y', labelsize=8)
        ax1.bar_label(bars1, fmt='{:,.0f}', padding=4, fontsize=9)
        ax1.set_xlim(right=best_dpav * 1.25 if best_dpav > 0 else 1)

        # 2. Total AV Plot
        df_av = df.sort_values(by="total_av", ascending=False)
        y_labels_av = [f"{r['team_name']} [{r['battle_mode']}]\n{r['datetime'][5:]}" for _, r in df_av.iterrows()]
        min_av = df_av['total_av'].min()
        colors_av = ['#2ecc71' if v == min_av else '#e74c3c' for v in df_av["total_av"]]

        bars2 = ax2.barh(y_labels_av, df_av["total_av"], color=colors_av, height=0.6)
        ax2.set_title("Action Value Comparison (AV - Lower is better)", fontsize=11, fontweight="bold")
        ax2.set_xlabel("Total Action Value (AV)")
        ax2.tick_params(axis='y', labelsize=8)
        ax2.bar_label(bars2, fmt='{:.2f}', padding=4, fontsize=9)
        max_av = df_av["total_av"].max()
        ax2.set_xlim(right=max_av * 1.25 if max_av > 0 else 1)

        self.fig_comparison.tight_layout()
        self.canvas_comparison.draw()

    def open_selected_detail(self):
        selected_items = self.tree.selection()
        if not selected_items:
            messagebox.showinfo("Hint", "Please select a battle session from the table first.")
            return

        idx = int(selected_items[0])
        if idx < len(self.current_filtered_data):
            self.show_detail_window(self.current_filtered_data[idx])

    def open_multi_run_comparison(self):
        selected_items = self.tree.selection()
        if len(selected_items) < 2:
            messagebox.showinfo("Multi-Run Comparison", "Please select at least 2 runs (hold Ctrl or Shift while clicking) to compare.")
            return

        selected_runs = [self.current_filtered_data[int(iid)] for iid in selected_items if int(iid) < len(self.current_filtered_data)]
        self.show_multi_comparison_window(selected_runs)

    # =========================================================================
    # SO SÁNH NHIỀU LẦN CHẠY TRÊN CÙNG MỘT ĐỒ THỊ (MULTI-RUN OVERLAY)
    # =========================================================================
    def show_multi_comparison_window(self, runs):
        comp_win = tk.Toplevel(self.master)
        comp_win.title(f"Multi-Run Comparison ({len(runs)} Runs Selected)")
        comp_win.geometry("1200x760")

        fig = Figure(figsize=(11.5, 7), dpi=100)
        canvas = FigureCanvasTkAgg(fig, master=comp_win)
        canvas.get_tk_widget().pack(fill=tk.BOTH, expand=True, padx=10, pady=10)

        ax1, ax2 = fig.subplots(1, 2)
        palette = ['#e74c3c', '#3498db', '#2ecc71', '#9b59b6', '#f39c12', '#1abc9c', '#34495e']

        # Biểu đồ 1: Chồng đường cong Cumulative Damage theo AV
        for idx, run in enumerate(runs):
            raw = run["raw_data"]
            av_hist = raw.get("av_history") or raw.get("turn_history", [])
            if av_hist:
                av_pts = [0.0] + [float(t.get("action_value", 0.0)) for t in av_hist]
                cum_dmg = [0.0]
                curr = 0.0
                for t in av_hist:
                    curr += float(t.get("total_damage", 0.0))
                    cum_dmg.append(curr)

                label = f"{run['team_name']} ({run['datetime'][5:]}) - {run['total_av']:.1f} AV"
                color = palette[idx % len(palette)]
                ax1.step(av_pts, cum_dmg, where='post', linewidth=2.2, label=label, color=color)

        ax1.set_title("Cumulative Damage vs Action Value (AV Overlay)", fontsize=11, fontweight="bold")
        ax1.set_xlabel("Elapsed Action Value (AV)")
        ax1.set_ylabel("Cumulative Damage")
        ax1.legend(fontsize=8, loc="upper left")

        # Biểu đồ 2: So sánh DpAV theo từng Wave giữa các lần chạy
        all_waves = sorted(list({
            m["wave"]
            for run in runs
            for m in self.calculate_wave_breakdown(run["raw_data"])[0]
        }))

        if all_waves:
            bar_width = 0.8 / max(1, len(runs))
            x_indices = range(len(all_waves))

            for idx, run in enumerate(runs):
                w_metrics, _, _ = self.calculate_wave_breakdown(run["raw_data"])
                w_dict = {m["wave"]: m["dpav"] for m in w_metrics}
                vals = [w_dict.get(w, 0.0) for w in all_waves]
                offsets = [x + (idx - len(runs) / 2 + 0.5) * bar_width for x in x_indices]
                label = f"{run['team_name']} ({run['datetime'][11:]})"
                bars = ax2.bar(offsets, vals, width=bar_width, label=label, color=palette[idx % len(palette)])
                ax2.bar_label(bars, fmt='{:,.0f}', padding=2, fontsize=7, rotation=45)

            ax2.set_xticks(list(x_indices))
            ax2.set_xticklabels([f"Wave {w}" for w in all_waves])
            ax2.set_title("Phase DpAV Comparison by Wave", fontsize=11, fontweight="bold")
            ax2.set_ylabel("Wave DpAV")
            ax2.legend(fontsize=8)

        fig.tight_layout()
        canvas.draw()

    # =========================================================================
    # TÍNH TOÁN METRICS CHI TIẾT TỪNG WAVE (PHASE)
    # =========================================================================
    def calculate_wave_breakdown(self, data):
        turn_history = data.get("turn_history", [])
        wave_map = {}

        for t_idx, turn in enumerate(turn_history):
            w = turn.get("wave", 1)
            if w not in wave_map:
                wave_map[w] = {
                    "turns": [],
                    "turn_indices": [],
                    "total_damage": 0.0,
                    "min_av": None,
                    "max_av": None,
                }
            wave_map[w]["turns"].append(turn)
            wave_map[w]["turn_indices"].append(t_idx + 1)
            wave_map[w]["total_damage"] += float(turn.get("total_damage", 0.0) or 0.0)

            av = float(turn.get("action_value", 0.0) or 0.0)
            if wave_map[w]["min_av"] is None or av < wave_map[w]["min_av"]:
                wave_map[w]["min_av"] = av
            if wave_map[w]["max_av"] is None or av > wave_map[w]["max_av"]:
                wave_map[w]["max_av"] = av

        sorted_waves = sorted(wave_map.keys())
        wave_metrics = []
        prev_end_av = 0.0
        wave_transition_avs = []
        wave_transition_turns = []

        total_battle_dmg = float(data.get("total_damage", 0.0) or 0.0)

        for w in sorted_waves:
            w_info = wave_map[w]
            start_av = prev_end_av
            end_av = w_info["max_av"] if w_info["max_av"] is not None else start_av
            # Nếu AV bị reset về 0 khi sang Wave mới (end_av < start_av), tính av_cost bằng chính end_av của wave đó
            av_cost = (end_av - start_av) if end_av >= start_av else end_av
            dmg = w_info["total_damage"]
            dpav = (dmg / av_cost) if av_cost > 0 else dmg
            share_pct = (dmg / total_battle_dmg * 100.0) if total_battle_dmg > 0 else 0.0

            wave_metrics.append({
                "wave": w,
                "av_cost": av_cost,
                "damage": dmg,
                "dpav": dpav,
                "share_pct": share_pct,
                "start_av": start_av,
                "end_av": end_av,
                "turns_count": len(w_info["turns"]),
                "last_turn_idx": w_info["turn_indices"][-1]
            })

            prev_end_av = end_av
            if w != sorted_waves[-1]:
                wave_transition_avs.append(end_av)
                wave_transition_turns.append(w_info["turn_indices"][-1])

        return wave_metrics, wave_transition_avs, wave_transition_turns

    def show_detail_window(self, summary_info):
        data = summary_info["raw_data"]
        detail_window = tk.Toplevel(self.master)
        detail_window.title(f"Battle Analysis: {summary_info['session_id']}")
        detail_window.geometry("1220x920")

        wave_metrics, wave_trans_avs, wave_trans_turns = self.calculate_wave_breakdown(data)

        table_frame = ttk.LabelFrame(detail_window, text="Wave / Phase Performance Breakdown", padding=8)
        table_frame.pack(fill=tk.X, padx=10, pady=(10, 5))

        tree_wave = ttk.Treeview(
            table_frame,
            columns=("phase", "turns", "av_cost", "damage", "share", "dpav"),
            show="headings",
            height=min(6, len(wave_metrics) + 1)
        )
        tree_wave.heading("phase", text="Wave / Phase")
        tree_wave.heading("turns", text="Turns")
        tree_wave.heading("av_cost", text="AV Cost")
        tree_wave.heading("damage", text="Damage")
        tree_wave.heading("share", text="DMG Share (%)")
        tree_wave.heading("dpav", text="Phase DpAV")

        tree_wave.column("phase", width=140, anchor="center")
        tree_wave.column("turns", width=90, anchor="center")
        tree_wave.column("av_cost", width=120, anchor="e")
        tree_wave.column("damage", width=150, anchor="e")
        tree_wave.column("share", width=130, anchor="e")
        tree_wave.column("dpav", width=150, anchor="e")

        for m in wave_metrics:
            tree_wave.insert("", "end", values=(
                f"Wave {m['wave']}",
                f"{m['turns_count']}",
                f"{m['av_cost']:.2f} AV",
                f"{m['damage']:,.0f}",
                f"{m['share_pct']:.1f}%",
                f"{m['dpav']:,.1f}"
            ))

        total_dmg = float(data.get("total_damage", 0.0) or 0.0)
        total_av = float(data.get("total_av", 0.0) or 0.0)
        total_dpav = float(data.get("total_dpav", 0.0) or 0.0)
        total_turns = len(data.get("turn_history", []))

        tree_wave.insert("", "end", values=(
            "⭐ TOTAL (Whole Run)",
            f"{total_turns}",
            f"{total_av:.2f} AV",
            f"{total_dmg:,.0f}",
            "100.0%",
            f"{total_dpav:,.1f}"
        ))
        tree_wave.pack(fill=tk.X, expand=True)

        # Khung dưới: 4 biểu đồ trực quan
        chart_frame = ttk.Frame(detail_window)
        chart_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=(5, 10))

        fig = Figure(figsize=(11.5, 7.5), dpi=100)
        canvas = FigureCanvasTkAgg(fig, master=chart_frame)
        canvas.get_tk_widget().pack(fill=tk.BOTH, expand=True)

        (ax1, ax2), (ax3, ax4) = fig.subplots(2, 2)
        fig.suptitle(
            f"Run Breakdown: {summary_info['team_name']} | {summary_info['battle_mode']} (Stage {summary_info['stage_id']})",
            fontsize=12, fontweight="bold"
        )

        characters = data.get("characters", {})
        char_names = list(characters.keys())
        char_damages = [float(d.get('total_damage', 0.0) or 0.0) for d in characters.values()]
        palette = ['#e74c3c', '#3498db', '#f1c40f', '#2ecc71', '#9b59b6', '#e67e22', '#1abc9c', '#34495e']

        # 1. Damage Distribution Pie
        if sum(char_damages) > 0:
            ax1.pie(
                char_damages, labels=char_names, autopct='%1.1f%%', startangle=140,
                colors=palette[:len(char_names)], textprops={'fontsize': 9}
            )
        ax1.set_title("Character Damage Distribution (%)", fontsize=10, fontweight="bold")

        # 2. Total Damage & Character DpAV Bar
        if char_damages:
            bars = ax2.bar(char_names, char_damages, color=palette[:len(char_names)], width=0.5)
            ax2.set_title("Total Damage & DpAV by Character", fontsize=10, fontweight="bold")
            ax2.set_ylabel("Damage")
            ax2.tick_params(axis='x', rotation=20, labelsize=9)
            custom_labels = [
                f"{dmg:,.0f}\n({(dmg / total_av):,.0f} DpAV)" if total_av > 0 else f"{dmg:,.0f}"
                for dmg in char_damages
            ]
            ax2.bar_label(bars, labels=custom_labels, padding=3, fontsize=8)
            max_c_dmg = max(char_damages)
            ax2.set_ylim(top=max_c_dmg * 1.32 if max_c_dmg > 0 else 1)

        # 3. Damage Per Turn (Đã fix lỗi lệch kích thước mảng khi có Memosprite/Summon)
        turn_history = data.get("turn_history", [])
        if turn_history:
            turns = list(range(1, len(turn_history) + 1))
            total_turn_dmg = [float(t.get('total_damage', 0.0) or 0.0) for t in turn_history]
            ax3.plot(turns, total_turn_dmg, marker='o', color='#2c3e50', linewidth=2, label='Team Turn DMG')

            lineup = data.get("lineup", [])
            for i, c_name in enumerate(lineup):
                c_dmg = [
                    float(t.get('avatars_turn_damage', [])[i]) if i < len(t.get('avatars_turn_damage', [])) else 0.0
                    for t in turn_history
                ]
                if any(v > 0 for v in c_dmg):
                    ax3.plot(turns, c_dmg, linestyle='--', marker='.', alpha=0.65, label=c_name)

            max_t_dmg = max(total_turn_dmg) if total_turn_dmg else 1.0
            for w_t in wave_trans_turns:
                ax3.axvline(x=w_t + 0.5, color='#e74c3c', linestyle=':', linewidth=1.5, alpha=0.8)
                ax3.text(w_t + 0.55, max_t_dmg * 0.85, "Next Wave", color='#e74c3c', fontsize=8, rotation=90)

            ax3.set_title("Damage Per Turn (with Wave Boundaries)", fontsize=10, fontweight="bold")
            ax3.set_xlabel("Turn Order")
            ax3.set_ylabel("Damage")
            ax3.legend(fontsize=7, loc='upper left')

        # 4. Cumulative Damage over Action Value (Fallback sang turn_history nếu av_history trống)
        av_history = data.get("av_history") or turn_history
        if av_history:
            av_points = [float(turn.get("action_value", 0.0) or 0.0) for turn in av_history]
            cum_dmg = []
            curr = 0.0
            for turn in av_history:
                curr += float(turn.get("total_damage", 0.0) or 0.0)
                cum_dmg.append(curr)

            ax4.step(av_points, cum_dmg, where='post', color='#e67e22', linewidth=2.2, label='Cumulative DMG')
            ax4.fill_between(av_points, cum_dmg, step='post', alpha=0.15, color='#e67e22')

            for w_idx, w_av in enumerate(wave_trans_avs):
                ax4.axvline(x=w_av, color='#c0392b', linestyle='--', linewidth=1.5, alpha=0.85)
                ax4.text(w_av + 1.0, curr * 0.2, f"Wave {w_idx+1} End\n({w_av:.1f} AV)", color='#c0392b', fontsize=8)

            ax4.set_title("Cumulative Damage Curve (over Action Value)", fontsize=10, fontweight="bold")
            ax4.set_xlabel("Elapsed Action Value (AV)")
            ax4.set_ylabel("Total Damage")
            ax4.legend(fontsize=8, loc='upper left')
        else:
            ax4.text(0.5, 0.5, "No AV History Available", ha='center', va='center', transform=ax4.transAxes)

        fig.tight_layout()
        canvas.draw()


if __name__ == "__main__":
    root = tk.Tk()
    app = ModernDamageAnalyzer(root)
    root.mainloop()