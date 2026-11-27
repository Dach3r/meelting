module Molecules
  module CountPill
    class Component < ViewComponent::Base
      TONES = {
        neutral: { box: "bg-graphite-50 text-graphite-900", count: "text-sun-400" },
        wait: { box: "bg-sun-400 text-graphite-900", count: "text-sun-400" },
        run: { box: "bg-sky-100 text-sky-700", count: "text-sky-400" }
      }.freeze

      def initialize(count:, label:, href:, tone: :neutral)
        @count = count
        @label = label
        @href = href
        @tone = TONES.fetch(tone)
      end

      private
      attr_reader :count, :label, :href, :tone
    end
  end
end
