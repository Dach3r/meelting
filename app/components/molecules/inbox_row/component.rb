module Molecules
  module InboxRow
    class Component < ViewComponent::Base
      STATE_CHIPS = {
        awaiting_approval: { tone: :wait },
        ready_to_merge: { tone: :ok, icon: :check },
        conflicts: { tone: :bad, icon: :triangle_alert },
        running: { tone: :run },
        assigned: { tone: :run }
      }.freeze

      def initialize(item:, href: nil, selected: false)
        @item = item
        @href = href
        @selected = selected
      end

      private
      attr_reader :item, :href, :selected

      def chip
        STATE_CHIPS[item.state]
      end

      def chip_label
        case item.state
        when :running then helpers.format_duration(item.duration_seconds)
        when :assigned then t("home.states.assigned", number: item.detail(:run_number))
        else t("home.states.#{item.state}")
        end
      end

      def subtitle
        case item.kind
        when :run then run_subtitle
        when :pull_request then pull_request_subtitle
        else t("home.subtitles.issue", number: item.number)
        end
      end

      def run_subtitle
        return item.detail(:command) if item.running?

        t("home.subtitles.run", number: item.number, files: item.detail(:files), commands: item.detail(:commands))
      end

      def pull_request_subtitle
        if item.state == :conflicts
          t("home.subtitles.pull_request_conflicts", number: item.number, file: item.detail(:conflict_file))
        else
          t("home.subtitles.pull_request_ready", number: item.number, tests: item.detail(:tests))
        end
      end

      def unassigned?
        item.state == :unassigned
      end
    end
  end
end
