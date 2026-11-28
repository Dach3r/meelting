module Molecules
  module SettingRow
    class Component < ViewComponent::Base
      renders_one :hint

      def initialize(label:, label_for: nil, id: nil, badge: nil)
        @label = label
        @label_for = label_for
        @id = id
        @badge = badge
      end

      private
      attr_reader :label, :label_for, :id, :badge

      def label_tag_name
        label_for ? :label : :span
      end
    end
  end
end
