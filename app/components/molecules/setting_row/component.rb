module Molecules
  module SettingRow
    class Component < ViewComponent::Base
      renders_one :hint

      def initialize(label:, label_for: nil, id: nil)
        @label = label
        @label_for = label_for
        @id = id
      end

      private
      attr_reader :label, :label_for, :id

      def label_tag_name
        label_for ? :label : :span
      end
    end
  end
end
